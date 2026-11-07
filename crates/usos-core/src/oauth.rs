//! OAuth 1.0a (RFC 5849) HMAC-SHA1 request signing.
//!
//! Everything here is a pure function except [`generate_nonce`] and
//! [`unix_timestamp`]; callers inject the nonce and timestamp so signatures
//! are reproducible in tests. The consumer key source is abstracted behind
//! [`ConsumerSigner`] so it can later be replaced (e.g. by a remote signer)
//! without touching the client.

use std::fmt;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sha1::Sha1;
use url::Url;

use crate::secret::Secret;

/// RFC 5849 §3.6: everything except ALPHA / DIGIT / `-` / `.` / `_` / `~` is escaped.
const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encode per RFC 5849 §3.6 (uppercase hex, unreserved set only).
pub fn percent_encode(input: &str) -> String {
    utf8_percent_encode(input, ENCODE_SET).to_string()
}

/// RFC 5849 §3.4.1.2: `scheme://host[:port]path`, lowercase scheme/host, default port and query dropped.
pub fn base_string_uri(url: &Url) -> String {
    let mut out = format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default());
    if let Some(port) = url.port() {
        out.push(':');
        out.push_str(&port.to_string());
    }
    out.push_str(url.path());
    out
}

/// RFC 5849 §3.4.1.3: the normalized parameter string.
///
/// Collects the URL's query parameters, `body` (decoded form parameters) and
/// `oauth` parameters, drops `oauth_signature`, encodes names and values,
/// sorts by name then value, and joins with `&`.
pub fn normalized_parameters(
    url: &Url,
    body: &[(String, String)],
    oauth: &[(String, String)],
) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .chain(body.iter().cloned())
        .chain(oauth.iter().cloned())
        .filter(|(k, _)| k != "oauth_signature")
        .map(|(k, v)| (percent_encode(&k), percent_encode(&v)))
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// RFC 5849 §3.4.1: `METHOD&encoded-uri&encoded-normalized-parameters`.
pub fn signature_base_string(
    method: &str,
    url: &Url,
    body: &[(String, String)],
    oauth: &[(String, String)],
) -> String {
    format!(
        "{}&{}&{}",
        method.to_ascii_uppercase(),
        percent_encode(&base_string_uri(url)),
        percent_encode(&normalized_parameters(url, body, oauth)),
    )
}

/// RFC 5849 §3.4.2: `encode(consumer_secret) & encode(token_secret)`.
pub fn signing_key(consumer_secret: &str, token_secret: &str) -> String {
    format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    )
}

/// Base64 HMAC-SHA1 of `base_string` under `key`.
pub fn hmac_sha1_signature(base_string: &str, key: &str) -> String {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(key.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(base_string.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// Where the consumer key and signature come from.
///
/// The default implementation, [`StaticConsumer`], holds the secret in memory.
/// Another implementation could forward `base_string` to a remote signer so the
/// consumer secret never ships with the client.
pub trait ConsumerSigner: Send + Sync + fmt::Debug {
    fn consumer_key(&self) -> &str;
    /// Sign `base_string`; `token_secret` is `None` for two-legged requests.
    fn sign(&self, base_string: &str, token_secret: Option<&str>) -> String;
    /// Remove this signer's secrets from `text` (used to scrub error messages).
    fn scrub(&self, text: &str) -> String;
}

/// A consumer key and secret held in memory.
#[derive(Clone)]
pub struct StaticConsumer {
    key: Secret,
    secret: Secret,
}

impl StaticConsumer {
    pub fn new(key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            key: Secret::new(key),
            secret: Secret::new(secret),
        }
    }
}

impl fmt::Debug for StaticConsumer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticConsumer")
            .field("key", &self.key)
            .field("secret", &self.secret)
            .finish()
    }
}

impl ConsumerSigner for StaticConsumer {
    fn consumer_key(&self) -> &str {
        self.key.expose()
    }

    fn sign(&self, base_string: &str, token_secret: Option<&str>) -> String {
        let key = signing_key(self.secret.expose(), token_secret.unwrap_or(""));
        hmac_sha1_signature(base_string, &key)
    }

    fn scrub(&self, text: &str) -> String {
        let mut out = text.to_owned();
        for s in [self.secret.expose(), self.key.expose()] {
            if !s.is_empty() {
                out = out.replace(s, "***");
            }
        }
        out
    }
}

/// Everything needed to sign one request.
/// Deliberately not `Debug`: it borrows the token secret.
#[derive(Clone)]
pub struct SignParams<'a> {
    pub method: &'a str,
    /// Full URL including any query string.
    pub url: &'a Url,
    /// Decoded `application/x-www-form-urlencoded` body parameters.
    pub body: &'a [(String, String)],
    /// `(token, token_secret)` for three-legged requests.
    pub token: Option<(&'a str, &'a str)>,
    /// Extra `oauth_*` parameters such as `oauth_callback` or `oauth_verifier`.
    pub extra_oauth: &'a [(String, String)],
    pub nonce: &'a str,
    pub timestamp: u64,
}

/// The `oauth_*` parameters of a request, including `oauth_signature`, sorted by name.
pub fn signed_oauth_params(
    signer: &dyn ConsumerSigner,
    p: &SignParams<'_>,
) -> Vec<(String, String)> {
    let mut oauth: Vec<(String, String)> = vec![
        (
            "oauth_consumer_key".into(),
            signer.consumer_key().to_owned(),
        ),
        ("oauth_nonce".into(), p.nonce.to_owned()),
        ("oauth_signature_method".into(), "HMAC-SHA1".into()),
        ("oauth_timestamp".into(), p.timestamp.to_string()),
        ("oauth_version".into(), "1.0".into()),
    ];
    if let Some((token, _)) = p.token {
        oauth.push(("oauth_token".into(), token.to_owned()));
    }
    oauth.extend(p.extra_oauth.iter().cloned());
    let base = signature_base_string(p.method, p.url, p.body, &oauth);
    let signature = signer.sign(&base, p.token.map(|(_, secret)| secret));
    oauth.push(("oauth_signature".into(), signature));
    oauth.sort();
    oauth
}

/// The `Authorization: OAuth ...` header value for a request.
pub fn authorization_header(signer: &dyn ConsumerSigner, p: &SignParams<'_>) -> String {
    let params = signed_oauth_params(signer, p);
    let joined = params
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("OAuth {joined}")
}

/// 32 hex characters of OS randomness.
pub fn generate_nonce() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("operating system random number generator unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Seconds since the Unix epoch.
pub fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn percent_encoding_edge_cases() {
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("~"), "~");
        assert_eq!(percent_encode("-._~"), "-._~");
        assert_eq!(percent_encode("*"), "%2A");
        assert_eq!(percent_encode("+"), "%2B");
        assert_eq!(percent_encode("ś"), "%C5%9B");
        assert_eq!(percent_encode("a/b?c=d&e"), "a%2Fb%3Fc%3Dd%26e");
        assert_eq!(percent_encode("100%"), "100%25");
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn base_uri_normalization() {
        let u = Url::parse("HTTP://Example.COM:80/r%20v/X?id=123#frag").unwrap();
        assert_eq!(base_string_uri(&u), "http://example.com/r%20v/X");
        let u = Url::parse("https://example.com:8443").unwrap();
        assert_eq!(base_string_uri(&u), "https://example.com:8443/");
    }

    /// RFC 5849 §3.4.1 example: the base string must match the RFC text exactly.
    #[test]
    fn rfc5849_base_string_example() {
        let url = Url::parse("http://example.com/request?b5=%3D%253D&a3=a&c%40=&a2=r%20b").unwrap();
        // Body "c2&a3=2+q" decoded as a form.
        let body: Vec<(String, String)> = url::form_urlencoded::parse(b"c2&a3=2+q")
            .into_owned()
            .collect();
        let oauth = pairs(&[
            ("oauth_consumer_key", "9djdj82h48djs9d2"),
            ("oauth_token", "kkk9d7dh3k39sjv7"),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_timestamp", "137131201"),
            ("oauth_nonce", "7d8f3e4a"),
            ("oauth_signature", "bYT5CMsGcbgUdFHObYMEfcx6bsw%3D"),
        ]);
        let expected = "POST&http%3A%2F%2Fexample.com%2Frequest&a2%3Dr%2520b%26a3%3D2%2520q\
%26a3%3Da%26b5%3D%253D%25253D%26c%2540%3D%26c2%3D%26oauth_consumer_key%3D9djdj82h48djs9d2\
%26oauth_nonce%3D7d8f3e4a%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D137131201\
%26oauth_token%3Dkkk9d7dh3k39sjv7";
        assert_eq!(signature_base_string("POST", &url, &body, &oauth), expected);
    }

    /// Twitter's "Creating a signature" documentation vector.
    #[test]
    fn twitter_signature_vector() {
        let url =
            Url::parse("https://api.twitter.com/1.1/statuses/update.json?include_entities=true")
                .unwrap();
        let body = pairs(&[(
            "status",
            "Hello Ladies + Gentlemen, a signed OAuth request!",
        )]);
        let consumer = StaticConsumer::new(
            "xvz1evFS4wEEPTGEFPHBog",
            "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
        );
        let params = SignParams {
            method: "post",
            url: &url,
            body: &body,
            token: Some((
                "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
                "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE",
            )),
            extra_oauth: &[],
            nonce: "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg",
            timestamp: 1318622958,
        };
        let oauth = signed_oauth_params(&consumer, &params);
        let signature = oauth
            .iter()
            .find(|(k, _)| k == "oauth_signature")
            .map(|(_, v)| v.as_str());
        assert_eq!(signature, Some("hCtSmYh+iHYCEqBWrE7C7hYmtUk="));

        let header = authorization_header(&consumer, &params);
        assert!(header.starts_with("OAuth oauth_consumer_key=\"xvz1evFS4wEEPTGEFPHBog\", "));
        assert!(header.contains("oauth_signature=\"hCtSmYh%2BiHYCEqBWrE7C7hYmtUk%3D\""));
        assert!(header.contains("oauth_version=\"1.0\""));
    }

    #[test]
    fn twitter_base_string_and_key() {
        let url =
            Url::parse("https://api.twitter.com/1.1/statuses/update.json?include_entities=true")
                .unwrap();
        let body = pairs(&[(
            "status",
            "Hello Ladies + Gentlemen, a signed OAuth request!",
        )]);
        let oauth = pairs(&[
            ("oauth_consumer_key", "xvz1evFS4wEEPTGEFPHBog"),
            ("oauth_nonce", "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg"),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_timestamp", "1318622958"),
            (
                "oauth_token",
                "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
            ),
            ("oauth_version", "1.0"),
        ]);
        let expected = "POST&https%3A%2F%2Fapi.twitter.com%2F1.1%2Fstatuses%2Fupdate.json&include_entities%3Dtrue%26oauth_consumer_key%3Dxvz1evFS4wEEPTGEFPHBog%26oauth_nonce%3DkYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D1318622958%26oauth_token%3D370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb%26oauth_version%3D1.0%26status%3DHello%2520Ladies%2520%252B%2520Gentlemen%252C%2520a%2520signed%2520OAuth%2520request%2521";
        assert_eq!(signature_base_string("POST", &url, &body, &oauth), expected);
        assert_eq!(
            signing_key(
                "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
                "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE"
            ),
            "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw&LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE"
        );
    }

    #[test]
    fn two_legged_request_has_no_token_and_empty_secret_suffix() {
        let consumer = StaticConsumer::new("ck", "cs");
        let url = Url::parse("https://example.com/services/oauth/request_token").unwrap();
        let body = pairs(&[("scopes", "studies|grades")]);
        let extra = pairs(&[("oauth_callback", "http://127.0.0.1:1/callback")]);
        let params = SignParams {
            method: "POST",
            url: &url,
            body: &body,
            token: None,
            extra_oauth: &extra,
            nonce: "n",
            timestamp: 1,
        };
        let header = authorization_header(&consumer, &params);
        assert!(!header.contains("oauth_token"));
        assert!(header.contains("oauth_callback=\"http%3A%2F%2F127.0.0.1%3A1%2Fcallback\""));
        let oauth = signed_oauth_params(&consumer, &params);
        let unsigned: Vec<_> = oauth
            .iter()
            .filter(|(k, _)| k != "oauth_signature")
            .cloned()
            .collect();
        let base = signature_base_string("POST", &url, &body, &unsigned);
        let expected = hmac_sha1_signature(&base, "cs&");
        assert_eq!(
            oauth
                .iter()
                .find(|(k, _)| k == "oauth_signature")
                .unwrap()
                .1,
            expected
        );
    }

    #[test]
    fn debug_and_scrub_do_not_leak() {
        let consumer = StaticConsumer::new("the-key", "the-secret");
        let dbg = format!("{consumer:?}");
        assert!(!dbg.contains("the-key") && !dbg.contains("the-secret"));
        assert_eq!(consumer.scrub("a the-secret b the-key"), "a *** b ***");
    }

    #[test]
    fn nonce_is_hex_and_unique() {
        let a = generate_nonce();
        let b = generate_nonce();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
