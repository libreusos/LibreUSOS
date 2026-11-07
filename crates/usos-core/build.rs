//! Embeds the USOS consumer key and secret at compile time.
//!
//! The key and secret are resolved as a pair from one source, never mixed (half a pair counts
//! as absent; empty counts as unset):
//! 1. environment: `USOS_CONSUMER_KEY`, `USOS_CONSUMER_SECRET` (CI sets these as secrets);
//! 2. the workspace `.env` (`CARGO_MANIFEST_DIR/../../.env`): the same names, or the legacy
//!    `USOS_POLSL_CONSUMER_KEY` / `USOS_POLSL_CONSUMER_SECRET` (the CI variable names, see `.env.example`), or
//!    the legacy `CONSUMER_KEY` / `CONSUMER_SECRET`.
//!
//! After adding a `.env`, run `touch crates/usos-core/build.rs` (a missing file cannot be watched).
//!
//! `USOS_KEY_BASE_URL` (default PolSl) names the installation the key belongs to.
//! `USOS_NO_EMBEDDED_KEY=1` builds without any embedded key (for testing the "no key" path).
//! A missing key is not an error: the build succeeds and `embedded()` returns `None`.
//!
//! The values are written to a generated file in `OUT_DIR`, never to `cargo:` lines or
//! warnings, so they cannot show up in `cargo build -vv` output or CI logs.

use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};

const DEFAULT_BASE_URL: &str = "https://usosapi.polsl.pl/";

fn main() {
    for var in [
        "USOS_CONSUMER_KEY",
        "USOS_CONSUMER_SECRET",
        "USOS_POLSL_CONSUMER_KEY",
        "USOS_POLSL_CONSUMER_SECRET",
        "CONSUMER_KEY",
        "CONSUMER_SECRET",
        "USOS_KEY_BASE_URL",
        "USOS_NO_EMBEDDED_KEY",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dotenv_path = Path::new(&manifest).join("../../.env");
    // A missing file named in `rerun-if-changed` makes cargo rebuild on every call, so only watch
    // an existing `.env`. After adding one, run `touch crates/usos-core/build.rs`. Once any
    // `rerun-if-changed` is emitted the default "any package file" check is off, so watch
    // this script too.
    let dotenv_text = if dotenv_path.exists() {
        println!("cargo:rerun-if-changed={}", dotenv_path.display());
        println!("cargo:rerun-if-changed=build.rs");
        match fs::read_to_string(&dotenv_path) {
            Ok(text) => Some(text),
            Err(e) => {
                println!(
                    "cargo:warning=the workspace .env exists but cannot be read ({:?})",
                    e.kind()
                );
                None
            }
        }
    } else {
        None
    };
    let dotenv = dotenv_text.map(|t| parse_dotenv(&t)).unwrap_or_default();

    let from_env = |n: &str| env::var(n).ok();
    let from_dotenv = |n: &str| dotenv.get(n).cloned();
    // The key and secret come as a pair from one source; the sources are never mixed, and half a
    // pair counts as absent.
    let pair_from = |get: &dyn Fn(&str) -> Option<String>| -> Option<(String, String)> {
        let pick = |names: &[&str]| {
            names
                .iter()
                .filter_map(|n| get(n))
                .map(|v| v.trim().to_owned())
                .find(|v| !v.is_empty())
        };
        Some((
            pick(&["USOS_CONSUMER_KEY", "USOS_POLSL_CONSUMER_KEY", "CONSUMER_KEY"])?,
            pick(&["USOS_CONSUMER_SECRET", "USOS_POLSL_CONSUMER_SECRET", "CONSUMER_SECRET"])?,
        ))
    };

    let disabled = env::var("USOS_NO_EMBEDDED_KEY").is_ok_and(|v| !v.is_empty() && v != "0");
    let pair = if disabled {
        None
    } else {
        pair_from(&from_env).or_else(|| pair_from(&from_dotenv))
    };
    if pair.is_none() && !disabled {
        println!(
            "cargo:warning=no USOS consumer key embedded (set USOS_CONSUMER_KEY and USOS_CONSUMER_SECRET \
             for the build, or add a complete CONSUMER_KEY/CONSUMER_SECRET pair to the workspace .env \
             and run `touch crates/usos-core/build.rs`)"
        );
    }

    let base_url = [
        from_env("USOS_KEY_BASE_URL"),
        from_dotenv("USOS_KEY_BASE_URL"),
    ]
    .into_iter()
    .flatten()
    .map(|v| v.trim().to_owned())
    .find(|v| !v.is_empty())
    .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    if pair.is_some() && !base_url_is_valid(&base_url) {
        println!(
            "cargo:warning=USOS_KEY_BASE_URL is not a valid installation URL; the embedded key will not match any installation"
        );
    }

    let body = match pair {
        Some((key, secret)) => format!(
            "pub(crate) const EMBEDDED: Option<(&str, &str, &str)> = Some(({key:?}, {secret:?}, {base_url:?}));\n"
        ),
        None => "pub(crate) const EMBEDDED: Option<(&str, &str, &str)> = None;\n".to_owned(),
    };
    let out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("embedded_key.rs");
    fs::write(out, body).expect("write embedded_key.rs");
}

/// Mirrors `client::normalize_base_url`: https (or http on loopback), a host, no credentials.
fn base_url_is_valid(input: &str) -> bool {
    let with_scheme = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let Ok(url) = url::Url::parse(&with_scheme) else {
        return false;
    };
    let host = url.host_str().unwrap_or_default();
    let scheme_ok = url.scheme() == "https"
        || (url.scheme() == "http" && matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
    scheme_ok && !host.is_empty() && url.username().is_empty() && url.password().is_none()
}

/// A forgiving `KEY=VALUE` parser: comments, `export`, and single/double quotes.
fn parse_dotenv(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let value = match value.chars().next() {
            Some(q @ ('"' | '\'')) => match value[1..].find(q) {
                Some(end) => &value[1..=end],
                None => &value[1..],
            },
            _ => value.split(" #").next().unwrap_or(value).trim_end(),
        };
        map.insert(name.trim().to_owned(), value.to_owned());
    }
    map
}
