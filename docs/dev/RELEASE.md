# Releasing LibreUSOS

GitLab CI (`.gitlab-ci.yml`, scripts in `ci/`) builds on tags and on manual "Run pipeline" runs
(build only, no release). Every build needs the key variables; the Android build also needs the keystore and its variables.

## CI variables

Settings > CI/CD > Variables. Set all as masked and protected, and protect the tag pattern `v*`
(Settings > Repository > Protected tags), since protected variables only reach protected refs.

- `USOS_POLSL_CONSUMER_KEY`
- `USOS_POLSL_CONSUMER_SECRET`
- `LIBREUSOS_KEYSTORE_PASSWORD` (also used as the key password)
- `LIBREUSOS_KEY_ALIAS`

## Signing key

Create it once, then upload `libreusos-release.jks` under Settings > CI/CD > Secure Files (the name must match):

```sh
toolbox run -c android keytool -genkeypair -v -keystore ~/libreusos-release.jks \
  -alias libreusos -keyalg RSA -keysize 4096 -validity 10000
```

Back up the `.jks` and its passwords offline: if they are lost, users cannot update the app. Never commit them.

## Cutting a release

1. Bump `versionCode` / `versionName` in `android/app/build.gradle.kts` and merge to `main`.
2. Create the release in the GitLab UI (new tag `vX.Y.Z` on `main`), or push the tag with git.
3. The pipeline builds everything and the `release` job attaches the files to the release.

## Artifacts

- `usos-<version>-x86_64-linux.tar.gz`: static `usos` CLI
- `usos-<version>-x86_64-windows.zip`: `usos.exe`
- `libreusos-<version>.apk`: signed release APK (arm64-v8a, armeabi-v7a, x86_64)

`<version>` is the tag without the leading `v`. There is no macOS build (it needs the Apple SDK).
