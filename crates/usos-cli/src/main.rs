//! `usos`: command-line front end for LibreUSOS (unofficial USOS API client).

mod cache;
mod crstests;
mod grades;
mod login;
mod plan;
mod sync;

use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use usos_core::session::RevokeOutcome;
use usos_core::{Error as CoreError, Lang, Session, current_term};
use usos_store::RepoError;

use crate::cache::Data;

#[derive(Parser)]
#[command(
    name = "usos",
    version,
    about = "Unofficial USOS API client (LibreUSOS)"
)]
struct Cli {
    /// USOS API base URL (default: config file, then PolSl).
    #[arg(long, global = true, env = "USOS_BASE_URL")]
    base_url: Option<String>,
    /// Language for names (falls back to the other one when missing).
    #[arg(long, global = true, value_enum, default_value = "pl")]
    lang: LangArg,
    /// Use only the offline cache (plan, grades, tests); never contact USOS.
    #[arg(long, global = true)]
    offline: bool,
    /// Offline cache file (default: <data dir>/libreusos/cache.sqlite).
    #[arg(long, global = true, env = "LIBREUSOS_CACHE", value_name = "PATH")]
    cache: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum LangArg {
    Pl,
    En,
}

impl From<LangArg> for Lang {
    fn from(l: LangArg) -> Self {
        match l {
            LangArg::Pl => Lang::Pl,
            LangArg::En => Lang::En,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Log in through the browser (OAuth 1.0a) and store the access token.
    Login(LoginArgs),
    /// Show the logged-in user.
    Whoami,
    /// Make a signed API call: `usos raw users/user fields=id|first_name`.
    Raw {
        /// Method as `module/method`, e.g. `users/user`.
        method: String,
        /// Parameters as `key=value`.
        params: Vec<String>,
    },
    /// List your terms and mark the current one.
    Terms {
        /// Print only the id of the current term.
        #[arg(long)]
        current: bool,
    },
    /// Show your timetable (default: this week, Monday to Sunday).
    Plan(plan::PlanArgs),
    /// Show your grades (default: current term).
    Grades(grades::GradesArgs),
    /// Show your course tests, "sprawdziany" (default: current term).
    Tests(crstests::TestsArgs),
    /// Refresh the offline cache: plan, terms, and the current term's grades and tests.
    Sync(sync::SyncArgs),
    /// Delete the stored access token.
    Logout {
        /// Also revoke the token on the server (services/oauth/revoke_token).
        #[arg(long)]
        revoke: bool,
    },
}

#[derive(Args)]
pub struct LoginArgs {
    /// Skip the loopback callback and read the PIN from stdin.
    #[arg(long)]
    pub oob: bool,
    /// Also write the authorize URL to this file.
    #[arg(long, value_name = "PATH")]
    pub url_file: Option<std::path::PathBuf>,
    /// Run `<CMD> <url>` to open the URL (e.g. `xdg-open`).
    #[arg(long, value_name = "CMD")]
    pub open_cmd: Option<String>,
    /// How long to wait for the browser callback or the PIN.
    #[arg(long, default_value = "15m", value_parser = parse_duration)]
    pub timeout: Duration,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e:?}");
        // Exit directly: a blocked stdin read (`--oob`) must not hold the runtime open.
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Session::open(cli.base_url.as_deref())?;
    match cli.command {
        Command::Login(args) => login::run(&ctx, cli.cache, args).await,
        Command::Whoami => whoami(&ctx).await,
        Command::Raw { method, params } => raw(&ctx, &method, &params).await,
        Command::Terms { current } => terms(&ctx, cli.lang.into(), current).await,
        Command::Plan(args) => {
            let data = Data::open(&ctx, cli.cache, cli.offline).await?;
            plan::run(&data, cli.lang.into(), args).await
        }
        Command::Grades(args) => {
            let data = Data::open(&ctx, cli.cache, cli.offline).await?;
            grades::run(&data, cli.lang.into(), args).await
        }
        Command::Tests(args) => {
            let data = Data::open(&ctx, cli.cache, cli.offline).await?;
            crstests::run(&data, cli.lang.into(), args).await
        }
        Command::Sync(args) => sync::run(&ctx, cli.cache, args).await,
        Command::Logout { revoke } => logout(&ctx, cli.cache, revoke).await,
    }
}

/// Turn an error into a message with a hint for the common "token expired" case.
fn explain(e: CoreError, token_used: bool) -> anyhow::Error {
    match &e {
        CoreError::Api(api) if api.is_unauthorized() && token_used => {
            anyhow::anyhow!(
                "{api}\nhint: the access token may have expired or been revoked; run `usos login` again"
            )
        }
        CoreError::Api(api)
            if api.status == 400
                && api.code.as_deref() == Some("method_forbidden")
                && api.is_scope_missing() =>
        {
            anyhow::anyhow!(
                "{api}\nhint: the token lacks a required scope; run `usos login` again to grant the missing scopes"
            )
        }
        CoreError::NotLoggedIn => anyhow::anyhow!("not logged in; run `usos login` first"),
        _ => e.into(),
    }
}

/// Like [`explain`] for the store layer; `what` names the data a cache miss is about.
fn explain_repo(e: RepoError, what: &str) -> anyhow::Error {
    match e {
        RepoError::Core(e) => explain(e, true),
        RepoError::NotCached { .. } => {
            anyhow::anyhow!("{what} not cached; run `usos sync` while online (or drop --offline)")
        }
        RepoError::Store(e) => anyhow::Error::new(e).context("offline cache"),
    }
}

async fn whoami(ctx: &Session) -> Result<()> {
    let client = ctx.client()?;
    if !client.is_logged_in() {
        bail!(
            "not logged in to {}; run `usos login` first",
            ctx.base_url()
        );
    }
    let user = client.current_user().await.map_err(|e| explain(e, true))?;
    println!("{} (id {})", user.full_name(), user.id);
    Ok(())
}

async fn raw(ctx: &Session, method: &str, params: &[String]) -> Result<()> {
    let parsed: Vec<(String, String)> = params
        .iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .with_context(|| format!("parameter {p:?} is not in key=value form"))
        })
        .collect::<Result<_>>()?;
    let client = ctx.client()?;
    let value = client
        .call(method, &parsed)
        .await
        .map_err(|e| explain(e, client.is_logged_in()))?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

async fn terms(ctx: &Session, lang: Lang, only_current: bool) -> Result<()> {
    let client = ctx.client()?;
    let terms = client.user_terms().await.map_err(|e| explain(e, true))?;
    let today = chrono::Local::now().date_naive();
    let current = current_term(&terms, today).map(|t| t.id);
    if only_current {
        match current {
            Some(id) => println!("{id}"),
            None => bail!("no current term among your terms"),
        }
        return Ok(());
    }
    for t in &terms {
        let mark = if current.as_deref() == Some(t.id.as_str()) {
            "*"
        } else {
            " "
        };
        println!(
            "{mark} {:<12} {}  {} .. {}",
            t.id,
            t.name.get(lang).unwrap_or("-"),
            t.start_date,
            t.end_date
        );
    }
    Ok(())
}

async fn logout(ctx: &Session, cache: Option<std::path::PathBuf>, revoke: bool) -> Result<()> {
    // Logging out means leaving: the cached personal data goes too, even if the rest fails.
    let store = cache::open_store(cache)?;
    if usos_store::repo::forget_account(&store, ctx.base_url())
        .map_err(|e| explain_repo(e, "the cache"))?
    {
        println!("Offline cache of the account deleted.");
    }
    match ctx.logout(revoke).await? {
        RevokeOutcome::Revoked => println!("Token revoked on the server."),
        RevokeOutcome::NoToken => eprintln!("warning: no stored token to revoke"),
        RevokeOutcome::Failed(e) => {
            eprintln!("warning: could not revoke the token on the server: {e}")
        }
        RevokeOutcome::NotRequested => {}
    }
    println!("Logged out of {} (local token deleted).", ctx.base_url());
    Ok(())
}
