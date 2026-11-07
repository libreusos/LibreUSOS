//! A minimal one-shot HTTP server for the OAuth loopback callback (feature `loopback`).

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use crate::error::{LoginError, Result};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const PER_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the browser waits for the token exchange result.
const REPLY_WAIT: Duration = Duration::from_secs(30);

const PAGE_OK: &str = "<!doctype html><meta charset=\"utf-8\"><title>LibreUSOS</title>\
<body style=\"font-family:sans-serif;margin:3em\"><h1>LibreUSOS: login complete</h1>\
<p>You can close this tab.</p></body>";
const PAGE_FAILED: &str = "<!doctype html><meta charset=\"utf-8\"><title>LibreUSOS</title>\
<body style=\"font-family:sans-serif;margin:3em\"><h1>LibreUSOS: login failed</h1>\
<p>Authorization was received but the token exchange failed. See the terminal.</p></body>";
const PAGE_RECEIVED: &str = "<!doctype html><meta charset=\"utf-8\"><title>LibreUSOS</title>\
<body style=\"font-family:sans-serif;margin:3em\"><h1>LibreUSOS: authorization received</h1>\
<p>Return to the terminal.</p></body>";
const PAGE_DENIED: &str = "<!doctype html><meta charset=\"utf-8\"><title>LibreUSOS</title>\
<body style=\"font-family:sans-serif;margin:3em\"><h1>LibreUSOS: login was not completed</h1>\
<p>Authorization was not granted. Return to the terminal.</p></body>";
const PAGE_BAD: &str = "<!doctype html><meta charset=\"utf-8\"><title>LibreUSOS</title>\
<body style=\"font-family:sans-serif;margin:3em\"><h1>LibreUSOS: unexpected request</h1></body>";

/// What a single incoming request means for the login.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The expected callback with a verifier.
    Verifier(String),
    /// The expected request token, but no verifier (the user refused).
    Denied,
    /// Anything else (favicon, wrong token, junk): keep waiting.
    Ignore,
}

/// Classify a raw request head against the expected request token.
fn classify(request_head: &str, expected_token: &str) -> Verdict {
    let Some(line) = request_head.lines().next() else {
        return Verdict::Ignore;
    };
    let mut parts = line.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Verdict::Ignore;
    };
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Verdict::Ignore;
    };
    if url.path() != "/callback" {
        return Verdict::Ignore;
    }
    let get = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    if get("oauth_token").as_deref() != Some(expected_token) {
        return Verdict::Ignore;
    }
    match get("oauth_verifier") {
        Some(v) if !v.is_empty() => Verdict::Verifier(v),
        _ => Verdict::Denied,
    }
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let msg = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(msg.as_bytes()).await;
    let _ = stream.shutdown().await;
}

async fn read_head(stream: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while buf.len() < MAX_REQUEST_BYTES && !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    (!buf.is_empty()).then(|| String::from_utf8_lossy(&buf).into_owned())
}

/// A verifier received from the browser, with the browser's connection still open.
///
/// Call [`Pending::finish`] once the token exchange result is known so the
/// browser page can say whether it worked.
pub struct Pending {
    verifier: String,
    reply: oneshot::Sender<(bool, oneshot::Sender<()>)>,
}

impl Pending {
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// Tell the browser the outcome and wait (briefly) until the page is written.
    pub async fn finish(self, ok: bool) {
        let (done_tx, done_rx) = oneshot::channel();
        if self.reply.send((ok, done_tx)).is_ok() {
            let _ = tokio::time::timeout(PER_CONNECTION_TIMEOUT, done_rx).await;
        }
    }
}

type Outcome = std::result::Result<Pending, LoginError>;

/// Serve one connection; report a verifier or a denial through `tx`.
async fn handle(mut stream: TcpStream, expected_token: String, tx: mpsc::UnboundedSender<Outcome>) {
    let Ok(Some(head)) = tokio::time::timeout(PER_CONNECTION_TIMEOUT, read_head(&mut stream)).await
    else {
        return;
    };
    match classify(&head, &expected_token) {
        Verdict::Verifier(verifier) => {
            let (reply, reply_rx) = oneshot::channel();
            if tx.send(Ok(Pending { verifier, reply })).is_err() {
                respond(&mut stream, "200 OK", PAGE_RECEIVED).await;
                return;
            }
            match tokio::time::timeout(REPLY_WAIT, reply_rx).await {
                Ok(Ok((ok, done))) => {
                    if ok {
                        respond(&mut stream, "200 OK", PAGE_OK).await;
                    } else {
                        respond(&mut stream, "500 Internal Server Error", PAGE_FAILED).await;
                    }
                    let _ = done.send(());
                }
                _ => respond(&mut stream, "200 OK", PAGE_RECEIVED).await,
            }
        }
        Verdict::Denied => {
            respond(&mut stream, "400 Bad Request", PAGE_DENIED).await;
            let _ = tx.send(Err(LoginError::Denied));
        }
        Verdict::Ignore => respond(&mut stream, "404 Not Found", PAGE_BAD).await,
    }
}

/// Wait up to `timeout` for the browser to hit `/callback` with the expected
/// request token. Each connection is handled in its own task, so a silent
/// preconnect cannot stall the real callback.
pub async fn wait_for_verifier(
    listener: TcpListener,
    expected_token: &str,
    timeout: Duration,
) -> Result<Pending> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Outcome>();
    let serve = async {
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _) = accepted.map_err(LoginError::CallbackServer)?;
                    tokio::spawn(handle(stream, expected_token.to_owned(), tx.clone()));
                }
                outcome = rx.recv() => {
                    return match outcome {
                        Some(Ok(pending)) => Ok(pending),
                        Some(Err(e)) => Err(e.into()),
                        None => Err(LoginError::CallbackServerStopped.into()),
                    };
                }
            }
        }
    };
    match tokio::time::timeout(timeout, serve).await {
        Ok(result) => result,
        Err(_) => Err(LoginError::Timeout(timeout).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_requests() {
        let ok = "GET /callback?oauth_token=abc&oauth_verifier=12345 HTTP/1.1\r\nHost: x\r\n\r\n";
        assert_eq!(classify(ok, "abc"), Verdict::Verifier("12345".into()));
        assert_eq!(classify(ok, "other"), Verdict::Ignore);
        assert_eq!(
            classify("GET /favicon.ico HTTP/1.1\r\n\r\n", "abc"),
            Verdict::Ignore
        );
        assert_eq!(
            classify(
                "POST /callback?oauth_token=abc&oauth_verifier=1 HTTP/1.1\r\n\r\n",
                "abc"
            ),
            Verdict::Ignore
        );
        assert_eq!(
            classify("GET /callback?oauth_token=abc HTTP/1.1\r\n\r\n", "abc"),
            Verdict::Denied
        );
        assert_eq!(classify("", "abc"), Verdict::Ignore);
        assert_eq!(classify("garbage", "abc"), Verdict::Ignore);
    }

    #[tokio::test]
    async fn serves_one_callback() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(wait_for_verifier(listener, "tok", Duration::from_secs(5)));

        async fn get(port: u16, target: &str) -> String {
            let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            s.write_all(format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut out = String::new();
            s.read_to_string(&mut out).await.unwrap();
            out
        }
        // A silent preconnect must not stall the real callback.
        let _silent = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        assert!(get(port, "/favicon.ico").await.starts_with("HTTP/1.1 404"));
        assert!(
            get(port, "/callback?oauth_token=wrong&oauth_verifier=9")
                .await
                .starts_with("HTTP/1.1 404")
        );
        let browser =
            tokio::spawn(
                async move { get(port, "/callback?oauth_token=tok&oauth_verifier=777").await },
            );
        let pending = server.await.unwrap().unwrap();
        assert_eq!(pending.verifier(), "777");
        pending.finish(true).await;
        let page = browser.await.unwrap();
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("login complete"));
    }

    #[tokio::test]
    async fn times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let err = wait_for_verifier(listener, "tok", Duration::from_millis(50))
            .await
            .err()
            .unwrap();
        assert!(matches!(
            err,
            crate::error::Error::Login(LoginError::Timeout(_))
        ));
    }
}
