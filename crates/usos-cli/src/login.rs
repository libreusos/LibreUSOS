//! `usos login`: the terminal side of the OAuth 1.0a browser flow.
//! The flow itself lives in `usos_core::auth::login`.

use std::io::Write as _;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, BufReader};
use usos_core::Session;
use usos_core::auth::login::{Callback, CallbackMode, PendingLogin};

use crate::LoginArgs;

pub async fn run(ctx: &Session, cache: Option<std::path::PathBuf>, args: LoginArgs) -> Result<()> {
    let callback = if args.oob {
        Callback::Oob
    } else {
        Callback::Loopback
    };
    let mut pending = PendingLogin::begin(ctx, callback).await?;

    if !pending.dropped_scopes().is_empty() {
        eprintln!(
            "dropping scopes not defined by this installation: {}",
            pending.dropped_scopes().join(", ")
        );
    }
    if let Some(e) = pending.scope_list_error() {
        eprintln!(
            "warning: could not fetch the installation's scope list ({e}); requesting all scopes"
        );
    }
    println!("SCOPES_REQUESTED={}", pending.scopes().join("|"));
    if let Some(e) = pending.fallback_reason() {
        eprintln!("loopback callback was rejected by the server ({e}); falling back to oob");
    }
    println!("CALLBACK_MODE={}", pending.callback_mode());

    announce_authorize_url(&pending, &args).await?;

    let verifier = match pending.callback_mode() {
        CallbackMode::Loopback => {
            eprintln!(
                "Waiting for the browser callback (timeout {}) ...",
                humantime::format_duration(args.timeout)
            );
            pending.wait_loopback(args.timeout).await?
        }
        CallbackMode::Oob => read_pin(args.timeout).await?,
        CallbackMode::Custom => unreachable!("the CLI never requests a custom callback"),
    };

    let done = pending.complete(&verifier).await?;
    if let Some(user) = &done.user {
        println!("Logged in as {} (id {})", user.full_name(), user.id);
        // The offline cache now belongs to this user. A cache problem must not fail the login.
        let remembered = crate::cache::open_store(cache).and_then(|store| {
            usos_store::repo::remember_identity(&store, ctx.base_url(), user.clone())
                .map(|_| ())
                .map_err(Into::into)
        });
        if let Err(e) = remembered {
            eprintln!("warning: could not update the offline cache: {e}");
        }
    }
    if !done.granted_scopes.is_empty() {
        println!("Granted scopes: {}", done.granted_scopes.join(", "));
    }
    for warning in &done.warnings {
        eprintln!("warning: {warning}");
    }
    println!("Token stored in: {}", done.storage_backend);
    if let Some(reason) = done.keyring_error {
        eprintln!("note: the system keyring was unavailable ({reason}); used the file fallback");
    }
    Ok(())
}

async fn announce_authorize_url(pending: &PendingLogin, args: &LoginArgs) -> Result<()> {
    let url = pending.authorize_url();
    println!("AUTHORIZE_URL={url}");
    std::io::stdout().flush().ok();
    if let Some(path) = &args.url_file {
        write_private(path, &format!("{url}\n"))
            .with_context(|| format!("writing {}", path.display()))?;
    }
    if let Some(cmd) = &args.open_cmd {
        // `$1` keeps the URL as a single, unexpanded argument.
        let spawned = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{cmd} \"$1\""))
            .arg("sh")
            .arg(url.as_str())
            .spawn();
        if let Err(e) = spawned {
            eprintln!("warning: could not run --open-cmd: {e}");
        }
    }
    Ok(())
}

/// Write `text` to `path` with mode 0600 (the URL holds a live request token).
fn write_private(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    // `mode` only applies to newly created files.
    #[cfg(unix)]
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    file.write_all(text.as_bytes())
}

async fn read_pin(timeout: Duration) -> Result<String> {
    eprint!("Open the authorize URL, approve access, then enter the PIN shown: ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    let read = tokio::time::timeout(
        timeout,
        BufReader::new(tokio::io::stdin()).read_line(&mut line),
    )
    .await;
    match read {
        Err(_) => bail!(
            "timed out after {} waiting for the PIN",
            humantime::format_duration(timeout)
        ),
        Ok(Err(e)) => Err(e).context("reading the PIN from stdin"),
        Ok(Ok(0)) => bail!("no PIN provided (stdin closed)"),
        Ok(Ok(_)) => {
            let pin = line.trim();
            if pin.is_empty() {
                bail!("empty PIN");
            }
            Ok(pin.to_owned())
        }
    }
}
