//! The USOS API client: signed requests and the OAuth 1.0a login flow.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::error::{ApiError, Error, Result};
use crate::oauth::{self, ConsumerSigner, SignParams, StaticConsumer};
use crate::secret::Secret;

const USER_AGENT: &str = concat!("LibreUSOS/", env!("CARGO_PKG_VERSION"), " (unofficial)");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A temporary token from `services/oauth/request_token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestToken {
    pub token: Secret,
    pub secret: Secret,
}

/// A long-lived (with `offline_access`) token from `services/oauth/access_token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: Secret,
    pub secret: Secret,
}

/// Normalize a user-entered base URL: default to `https://`, drop query and
/// fragment, and end the path with `/`. Plain `http` is accepted only for loopback hosts.
pub fn normalize_base_url(input: &str) -> Result<Url> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidBaseUrl("empty".into()));
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let mut url = Url::parse(&with_scheme).map_err(|e| Error::InvalidBaseUrl(e.to_string()))?;
    let host = url.host_str().unwrap_or_default().to_owned();
    if host.is_empty() {
        return Err(Error::InvalidBaseUrl("missing host".into()));
    }
    match url.scheme() {
        "https" => {}
        "http" if matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]") => {}
        other => {
            return Err(Error::InvalidBaseUrl(format!(
                "scheme `{other}` is not allowed (USOS requires https)"
            )));
        }
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::InvalidBaseUrl(
            "credentials in the URL are not allowed".into(),
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// A client for one USOS installation.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    signer: Arc<dyn ConsumerSigner>,
    access: Option<AccessToken>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url.as_str())
            .field("logged_in", &self.access.is_some())
            .finish_non_exhaustive()
    }
}

/// The bundled Mozilla roots. Using them (`tls_certs_only`) keeps reqwest off the platform
/// verifier, which on Android needs a JNI init with an Android Context.
#[cfg(any(feature = "webpki-roots", target_os = "android"))]
fn bundled_roots() -> Result<Vec<reqwest::Certificate>> {
    webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|der| reqwest::Certificate::from_der(der).map_err(Error::from))
        .collect()
}

impl Client {
    /// A client using an in-memory consumer key and secret.
    pub fn new(base_url: &str, consumer_key: &str, consumer_secret: &str) -> Result<Self> {
        Self::with_signer(
            base_url,
            Arc::new(StaticConsumer::new(consumer_key, consumer_secret)),
        )
    }

    /// A client with a custom consumer-key source.
    pub fn with_signer(base_url: &str, signer: Arc<dyn ConsumerSigner>) -> Result<Self> {
        let builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        #[cfg(any(feature = "webpki-roots", target_os = "android"))]
        let builder = builder.tls_certs_only(bundled_roots()?);
        let http = builder.build()?;
        Ok(Self {
            http,
            base_url: normalize_base_url(base_url)?,
            signer,
            access: None,
        })
    }

    pub fn with_access_token(mut self, token: AccessToken) -> Self {
        self.access = Some(token);
        self
    }

    pub fn set_access_token(&mut self, token: Option<AccessToken>) {
        self.access = token;
    }

    /// Whether an access token is set.
    pub fn is_logged_in(&self) -> bool {
        self.access.is_some()
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    fn endpoint(&self, module_method: &str) -> Result<Url> {
        let name = module_method
            .trim()
            .trim_start_matches("services/")
            .trim_matches('/');
        let valid = !name.is_empty()
            && name.split('/').count() == 2
            && name.split('/').all(|seg| {
                !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
        if !valid {
            return Err(Error::InvalidMethod(module_method.to_owned()));
        }
        self.base_url
            .join(&format!("services/{name}"))
            .map_err(|e| Error::InvalidBaseUrl(e.to_string()))
    }

    /// Remove every known secret from `text`.
    fn scrub(&self, text: &str) -> String {
        let mut out = self.signer.scrub(text);
        if let Some(t) = &self.access {
            for s in [t.token.expose(), t.secret.expose()] {
                if !s.is_empty() {
                    out = out.replace(s, "***");
                }
            }
        }
        out
    }

    /// POST a signed, form-encoded request and return the raw success body.
    ///
    /// `params` go in the body and are part of the signature. `token` is
    /// `(token, secret)` of the request or access token, if any. `extra_oauth`
    /// holds `oauth_callback` / `oauth_verifier`. Every secret involved is scrubbed
    /// from error messages.
    async fn signed_post(
        &self,
        module_method: &str,
        params: &[(String, String)],
        token: Option<(&Secret, &Secret)>,
        extra_oauth: &[(String, String)],
    ) -> Result<String> {
        let url = self.endpoint(module_method)?;
        let nonce = oauth::generate_nonce();
        let header = oauth::authorization_header(
            self.signer.as_ref(),
            &SignParams {
                method: "POST",
                url: &url,
                body: params,
                token: token.map(|(t, s)| (t.expose(), s.expose())),
                extra_oauth,
                nonce: &nonce,
                timestamp: oauth::unix_timestamp(),
            },
        );
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();
        let response = self
            .http
            .post(url)
            .header(reqwest::header::AUTHORIZATION, header)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if status.is_success() {
            Ok(text)
        } else {
            let request_token_secrets: Vec<&str> = token
                .map(|(t, s)| vec![t.expose(), s.expose()])
                .unwrap_or_default();
            Err(ApiError::from_response(status.as_u16(), &text, |raw| {
                let mut out = self.scrub(raw);
                for s in &request_token_secrets {
                    if !s.is_empty() {
                        out = out.replace(s, "***");
                    }
                }
                out
            })
            .into())
        }
    }

    /// Step 1: `services/oauth/request_token`. `callback` is a URL or the literal `oob`.
    pub async fn request_token(&self, callback: &str, scopes: &[String]) -> Result<RequestToken> {
        let params = vec![("scopes".to_owned(), scopes.join("|"))];
        let extra = vec![("oauth_callback".to_owned(), callback.to_owned())];
        let body = self
            .signed_post("oauth/request_token", &params, None, &extra)
            .await?;
        let (token, secret) = parse_token_response(&body)?;
        Ok(RequestToken { token, secret })
    }

    /// Step 2: the URL the user must open in a browser.
    pub fn authorize_url(&self, request_token: &RequestToken) -> Url {
        let mut url = self
            .base_url
            .join("services/oauth/authorize")
            .expect("static path joins onto a base URL");
        url.query_pairs_mut()
            .append_pair("oauth_token", request_token.token.expose());
        url
    }

    /// Step 3: exchange the request token and verifier for an access token.
    pub async fn access_token(
        &self,
        request_token: &RequestToken,
        verifier: &str,
    ) -> Result<AccessToken> {
        let extra = vec![("oauth_verifier".to_owned(), verifier.trim().to_owned())];
        let body = self
            .signed_post(
                "oauth/access_token",
                &[],
                Some((&request_token.token, &request_token.secret)),
                &extra,
            )
            .await?;
        let (token, secret) = parse_token_response(&body)?;
        Ok(AccessToken { token, secret })
    }

    /// Generic signed call, e.g. `call("users/user", &[("fields".into(), "id|first_name".into())])`.
    /// Uses the access token when one is set, otherwise signs with the consumer only.
    pub async fn call(&self, method: &str, params: &[(String, String)]) -> Result<Value> {
        let token = self.access.as_ref().map(|t| (&t.token, &t.secret));
        let text = self.signed_post(method, params, token, &[]).await?;
        serde_json::from_str(&text)
            .map_err(|e| Error::UnexpectedResponse(format!("response is not JSON ({e})")))
    }

    /// Like [`Client::call`] but fails with [`Error::NotLoggedIn`] without an access token.
    pub async fn call_authenticated(
        &self,
        method: &str,
        params: &[(String, String)],
    ) -> Result<Value> {
        if self.access.is_none() {
            return Err(Error::NotLoggedIn);
        }
        self.call(method, params).await
    }

    /// `services/users/user` for the logged-in user with the given `fields` (pipe-separated).
    pub async fn users_user(&self, fields: &str) -> Result<Value> {
        self.call_authenticated("users/user", &[("fields".to_owned(), fields.to_owned())])
            .await
    }

    /// The scopes the current access token really has (`apisrv/consumer`, `token_scopes`).
    pub async fn token_scopes(&self) -> Result<Vec<String>> {
        let value = self
            .call_authenticated(
                "apisrv/consumer",
                &[("fields".to_owned(), "token_scopes".to_owned())],
            )
            .await?;
        match value.get("token_scopes") {
            Some(Value::Array(items)) => Ok(items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()),
            _ => Err(Error::UnexpectedResponse(
                "`token_scopes` missing or not a list".into(),
            )),
        }
    }

    /// `services/oauth/revoke_token` for the current access token.
    pub async fn revoke_token(&self) -> Result<()> {
        let token = self.access.as_ref().ok_or(Error::NotLoggedIn)?;
        self.signed_post(
            "oauth/revoke_token",
            &[],
            Some((&token.token, &token.secret)),
            &[],
        )
        .await?;
        Ok(())
    }

    /// Scope names the installation defines (anonymous GET `services/apiref/scopes`).
    pub async fn defined_scopes(&self) -> Result<Vec<String>> {
        let url = self
            .base_url
            .join("services/apiref/scopes")
            .expect("static path joins onto a base URL");
        let response = self.http.get(url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(ApiError::from_response(status.as_u16(), &text, |s| self.scrub(s)).into());
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| Error::UnexpectedResponse(format!("scope list is not JSON ({e})")))?;
        let items = value
            .as_array()
            .ok_or_else(|| Error::UnexpectedResponse("scope list is not an array".into()))?;
        Ok(items
            .iter()
            .filter_map(|i| i.get("key").and_then(Value::as_str).map(str::to_owned))
            .collect())
    }
}

/// Parse `oauth_token=...&oauth_token_secret=...` (form-encoded).
fn parse_token_response(body: &str) -> Result<(Secret, Secret)> {
    let map: HashMap<String, String> = url::form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    match (map.get("oauth_token"), map.get("oauth_token_secret")) {
        (Some(t), Some(s)) if !t.is_empty() && !s.is_empty() => {
            Ok((Secret::new(t.as_str()), Secret::new(s.as_str())))
        }
        _ => Err(Error::UnexpectedResponse(
            "response lacks oauth_token / oauth_token_secret".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_normalization() {
        assert_eq!(
            normalize_base_url("usosapi.polsl.pl").unwrap().as_str(),
            "https://usosapi.polsl.pl/"
        );
        assert_eq!(
            normalize_base_url(" https://usosapi.polsl.pl/?a=1#x ")
                .unwrap()
                .as_str(),
            "https://usosapi.polsl.pl/"
        );
        assert_eq!(
            normalize_base_url("https://example.com/usos")
                .unwrap()
                .as_str(),
            "https://example.com/usos/"
        );
        assert!(normalize_base_url("http://example.com").is_err());
        assert!(normalize_base_url("http://127.0.0.1:8080").is_ok());
        assert!(normalize_base_url("").is_err());
        assert!(normalize_base_url("https://u:p@example.com").is_err());
    }

    #[test]
    fn endpoint_building_and_validation() {
        let c = Client::new("https://example.com", "k", "s").unwrap();
        assert_eq!(
            c.endpoint("users/user").unwrap().as_str(),
            "https://example.com/services/users/user"
        );
        assert_eq!(
            c.endpoint("services/users/user").unwrap().as_str(),
            "https://example.com/services/users/user"
        );
        assert!(c.endpoint("users").is_err());
        assert!(c.endpoint("users/user?x=1").is_err());
        assert!(c.endpoint("../users/user").is_err());
        assert!(c.endpoint("a/b/c").is_err());
    }

    #[test]
    fn authorize_url_shape() {
        let c = Client::new("https://example.com/", "k", "s").unwrap();
        let t = RequestToken {
            token: Secret::new("abc"),
            secret: Secret::new("def"),
        };
        assert_eq!(
            c.authorize_url(&t).as_str(),
            "https://example.com/services/oauth/authorize?oauth_token=abc"
        );
    }

    #[test]
    fn token_response_parsing() {
        let (t, s) = parse_token_response(
            "oauth_token=a%2Fb&oauth_token_secret=s&oauth_callback_confirmed=true",
        )
        .unwrap();
        assert_eq!((t.expose(), s.expose()), ("a/b", "s"));
        assert!(parse_token_response("oauth_token=a").is_err());
        assert!(parse_token_response("").is_err());
    }

    #[test]
    fn debug_output_is_redacted() {
        let token = AccessToken {
            token: Secret::new("tok-123"),
            secret: Secret::new("sec-456"),
        };
        let c = Client::new("https://example.com", "key-789", "csec-000")
            .unwrap()
            .with_access_token(token.clone());
        for text in [format!("{c:?}"), format!("{token:?}"), format!("{c:#?}")] {
            for needle in ["tok-123", "sec-456", "key-789", "csec-000"] {
                assert!(!text.contains(needle), "leaked {needle}");
            }
        }
    }

    #[test]
    fn scrub_removes_all_known_secrets() {
        let token = AccessToken {
            token: Secret::new("tok-123"),
            secret: Secret::new("sec-456"),
        };
        let c = Client::new("https://example.com", "key-789", "csec-000")
            .unwrap()
            .with_access_token(token);
        assert_eq!(
            c.scrub("tok-123 sec-456 key-789 csec-000 ok"),
            "*** *** *** *** ok"
        );
    }
}
