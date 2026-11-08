# Windows x86_64 desktop app (GPUI, MSVC) -> dist/usos-desktop-<version>-x86_64-windows.zip.
# Runs natively on the GitLab SaaS Windows runner (tag saas-windows-medium-amd64), in PowerShell.
#
# Why not a Linux cross-build like the CLI: gpui-pre-windows compiles its HLSL shaders in build.rs with
# the Windows SDK's fxc.exe, and only when the build script itself runs on Windows. Cross-compiling fails
# with "couldn't find file .../shaders_bytes.rs" (checked with mingw); vkd3d-compiler cannot compile them.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Invoke-Native {
    param([scriptblock]$Command)
    # Windows PowerShell turns native stderr output (cargo, rustup progress) into errors under 'Stop'.
    $ErrorActionPreference = 'Continue'
    try { & $Command } finally { $ErrorActionPreference = 'Stop' }
    if ($LASTEXITCODE -ne 0) { throw "command failed with exit code ${LASTEXITCODE}: $Command" }
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Dist = Join-Path $Root 'dist'
New-Item -ItemType Directory -Force $Dist | Out-Null
Set-Location $Root

# Version, as in ci/common.sh: the tag without a leading "v", else git describe, else the short SHA.
$Version = $env:CI_COMMIT_TAG
if (-not $Version) {
    # Native stderr is an error under 'Stop' in Windows PowerShell; a failing git must fall through.
    $ErrorActionPreference = 'Continue'
    try { $Version = (git describe --tags --always 2>$null) } catch { $Version = $null }
    finally { $ErrorActionPreference = 'Stop' }
    if ($LASTEXITCODE -ne 0) { $Version = $null }
}
if (-not $Version) { $Version = if ($env:CI_COMMIT_SHORT_SHA) { $env:CI_COMMIT_SHORT_SHA } else { 'dev' } }
$Version = $Version -replace '^v', ''
$Name = "usos-desktop-$Version-x86_64-windows"

# Consumer key: required, except in merge request pipelines (protected variables may be missing there,
# and the artifacts must not carry the secret). The values are never printed.
if ($env:CI_PIPELINE_SOURCE -eq 'merge_request_event') {
    $env:USOS_NO_EMBEDDED_KEY = '1'
    Write-Host 'note: merge request pipeline, building without an embedded consumer key'
} else {
    $missing = @('USOS_POLSL_CONSUMER_KEY', 'USOS_POLSL_CONSUMER_SECRET') |
        Where-Object { -not (Get-Item "env:$_" -ErrorAction SilentlyContinue).Value }
    if ($missing) { throw "unset CI variable(s): $($missing -join ' ')" }
    $env:USOS_CONSUMER_KEY = $env:USOS_POLSL_CONSUMER_KEY
    $env:USOS_CONSUMER_SECRET = $env:USOS_POLSL_CONSUMER_SECRET
    $env:USOS_KEY_BASE_URL = 'https://usosapi.polsl.pl/'
}

# Cargo state inside the project dir, so the job cache can keep the registry.
$env:CARGO_HOME = Join-Path $Root '.ci\cargo'
$env:RUSTUP_HOME = Join-Path $Root '.ci\rustup'
$env:Path = "$env:CARGO_HOME\bin;$env:Path"

# Build tools the runner image may lack: MSVC with the Windows SDK (fxc.exe), NASM (aws-lc), CMake.
function Find-Fxc {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (Test-Path $kits) {
        Get-ChildItem $kits -Recurse -Filter fxc.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
    }
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$haveMsvc = (Test-Path $vswhere) -and (& $vswhere -latest -products '*' `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if (-not $haveMsvc -or -not (Find-Fxc)) {
    Invoke-Native { choco install -y visualstudio2022buildtools --no-progress --package-parameters `
        '--add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --includeRecommended --quiet --norestart' }
}
if (-not (Get-Command nasm -ErrorAction SilentlyContinue)) {
    Invoke-Native { choco install -y nasm --no-progress }
    $env:Path = "$env:ProgramFiles\NASM;$env:Path"
}
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    Invoke-Native { choco install -y cmake --no-progress }
    $env:Path = "$env:ProgramFiles\CMake\bin;$env:Path"
}
$fxc = Find-Fxc
if (-not $fxc) { throw 'fxc.exe not found: the Windows SDK is missing' }
$env:GPUI_FXC_PATH = $fxc
Write-Host "fxc: $fxc"

# Rust: the same version as the rust:* images in .gitlab-ci.yml (RUST_VERSION).
$RustVersion = if ($env:RUST_VERSION) { $env:RUST_VERSION } else { '1.99.1' }
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
    Invoke-Native { .\rustup-init.exe -y --default-toolchain none --profile minimal }
    Remove-Item rustup-init.exe
}
Invoke-Native { rustup toolchain install $RustVersion --profile minimal --component llvm-tools --target x86_64-pc-windows-msvc }
Invoke-Native { rustup default $RustVersion }
Invoke-Native { rustc -vV }

# Static CRT: no vcruntime140.dll or UCRT dependency. Paths are remapped as in ci/common.sh remap_flags.
$env:RUSTFLAGS = "--remap-path-prefix=$Root=/libreusos --remap-path-prefix=$env:CARGO_HOME=/cargo -C target-feature=+crt-static"
$env:CARGO_PROFILE_RELEASE_STRIP = 'symbols'
# The unit tests run on the Linux job; here the Windows code has to compile and link.
Invoke-Native { cargo build --release --locked -p usos-desktop --target x86_64-pc-windows-msvc }

$Exe = "target\x86_64-pc-windows-msvc\release\usos-desktop.exe"
# Only Windows system DLLs: no VC runtime, UCRT forwarders or MinGW libraries.
$Objdump = Join-Path (rustc --print sysroot) 'lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-objdump.exe'
$Dlls = & $Objdump -p $Exe | ForEach-Object { if ($_ -match '^\s*DLL Name:\s*(\S+)') { $Matches[1] } }
if ($LASTEXITCODE -ne 0) { throw "llvm-objdump failed on $Exe" }
$Dlls
$bad = $Dlls | Where-Object { $_ -match '^(vcruntime|msvcp|msvcr|concrt|vcomp|ucrtbase|api-ms-win-crt|lib|msys)' }
if ($bad) { throw "$Exe imports non-system DLL(s): $($bad -join ' ')" }

New-Item -ItemType Directory -Force "stage\$Name" | Out-Null
Copy-Item $Exe, LICENSE "stage\$Name\"
Copy-Item crates\usos-desktop\assets\NOTICE.md "stage\$Name\NOTICE-assets.md"
Copy-Item crates\usos-desktop\assets\fonts\OFL.txt "stage\$Name\LICENSE-OFL-Geist.txt"
Copy-Item crates\usos-desktop\assets\icons\LICENSE-APACHE-2.0.txt "stage\$Name\LICENSE-APACHE-2.0.txt"
Compress-Archive -Path "stage\$Name" -DestinationPath (Join-Path $Dist "$Name.zip") -Force
