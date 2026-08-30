use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use http::Uri;

const MAX_HTML_BYTES: u64 = 1024 * 1024;
const LOAD_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidUrl(String),
    UnsupportedTarget(String),
    Request(String),
    UnexpectedStatus(u16),
    UnsupportedContentType(String),
    InvalidBody(String),
}

impl LoadError {
    pub const fn is_invalid_input(&self) -> bool {
        matches!(self, Self::InvalidUrl(_) | Self::UnsupportedTarget(_))
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(reason) => write!(formatter, "invalid URL: {reason}"),
            Self::UnsupportedTarget(reason) => write!(formatter, "unsupported URL: {reason}"),
            Self::Request(reason) => write!(formatter, "page request failed: {reason}"),
            Self::UnexpectedStatus(status) => {
                write!(formatter, "page returned HTTP status {status}")
            }
            Self::UnsupportedContentType(value) => {
                write!(formatter, "page returned unsupported content type {value}")
            }
            Self::InvalidBody(reason) => write!(formatter, "page body could not be read: {reason}"),
        }
    }
}

pub(crate) fn load_local_html(value: &str) -> Result<String, LoadError> {
    let url = value
        .parse::<Uri>()
        .map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    validate_local_http_url(&url)?;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .proxy(None)
        .timeout_global(Some(LOAD_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| LoadError::Request(error.to_string()))?;

    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(LoadError::UnexpectedStatus(status));
    }
    if let Some(content_type) = response.headers().get("content-type") {
        let content_type = content_type
            .to_str()
            .map_err(|error| LoadError::InvalidBody(error.to_string()))?;
        let media_type = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if media_type != "text/html" && media_type != "application/xhtml+xml" {
            return Err(LoadError::UnsupportedContentType(content_type.into()));
        }
    }

    response
        .body_mut()
        .with_config()
        .limit(MAX_HTML_BYTES)
        .read_to_string()
        .map_err(|error| LoadError::InvalidBody(error.to_string()))
}

fn validate_local_http_url(url: &Uri) -> Result<(), LoadError> {
    if url.scheme_str() != Some("http") {
        return Err(LoadError::UnsupportedTarget(
            "only loopback HTTP pages are supported".into(),
        ));
    }
    let Some(authority) = url.authority() else {
        return Err(LoadError::InvalidUrl("the URL has no host".into()));
    };
    if authority.as_str().contains('@') {
        return Err(LoadError::UnsupportedTarget(
            "credentials are not accepted".into(),
        ));
    }

    let host = url.host().unwrap_or_default();
    let host_without_brackets = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host_without_brackets
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !is_loopback {
        return Err(LoadError::UnsupportedTarget(
            "the host must be localhost or a loopback IP address".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LoadError, validate_local_http_url};
    use http::Uri;

    #[test]
    fn accepts_loopback_http_urls() {
        for value in [
            "http://localhost:3000/page",
            "http://127.0.0.1:3000/page",
            "http://[::1]:3000/page",
        ] {
            assert!(validate_local_http_url(&value.parse::<Uri>().unwrap()).is_ok());
        }
    }

    #[test]
    fn rejects_non_loopback_targets() {
        let result = validate_local_http_url(&"http://example.com".parse::<Uri>().unwrap());

        assert!(matches!(result, Err(LoadError::UnsupportedTarget(_))));
    }

    #[test]
    fn rejects_https_before_network_access() {
        let result = validate_local_http_url(&"https://localhost".parse::<Uri>().unwrap());

        assert!(matches!(result, Err(LoadError::UnsupportedTarget(_))));
    }
}
