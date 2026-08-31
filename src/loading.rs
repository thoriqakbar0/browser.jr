use std::fmt;
use std::net::IpAddr;

use http::Uri;

const MAX_HTML_BYTES: u64 = 1024 * 1024;

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

    let agent = local_http_agent();
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

pub(crate) fn resolve_url_reference(base: &str, reference: &str) -> Result<String, LoadError> {
    let (reference, fragment) = reference
        .split_once('#')
        .map_or((reference, None), |(reference, fragment)| {
            (reference, Some(fragment))
        });
    let base = base
        .parse::<Uri>()
        .map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    let mut resolved = if let Ok(absolute) = reference.parse::<Uri>()
        && absolute.scheme().is_some()
    {
        absolute.to_string()
    } else {
        let scheme = base
            .scheme_str()
            .ok_or_else(|| LoadError::InvalidUrl("the base URL has no scheme".into()))?;
        if reference.starts_with("//") {
            format!("{scheme}:{reference}")
        } else {
            let authority = base
                .authority()
                .ok_or_else(|| LoadError::InvalidUrl("the base URL has no authority".into()))?;
            let base_path = base
                .path_and_query()
                .map(|value| value.path())
                .unwrap_or("/");
            let base_path_and_query = base.path_and_query().map_or("/", |value| value.as_str());
            let path_and_query = resolve_path_and_query(base_path, base_path_and_query, reference);
            format!("{scheme}://{authority}{path_and_query}")
        }
    };
    if let Some(fragment) = fragment {
        resolved.push('#');
        resolved.push_str(fragment);
    }
    Ok(resolved)
}

fn resolve_path_and_query(base_path: &str, base_path_and_query: &str, reference: &str) -> String {
    if reference.is_empty() {
        return base_path_and_query.into();
    }
    if reference.starts_with('?') {
        return format!("{base_path}{reference}");
    }
    let (reference_path, query) = reference
        .split_once('?')
        .map_or((reference, None), |(path, query)| (path, Some(query)));
    let joined = if reference_path.starts_with('/') {
        reference_path.into()
    } else {
        let directory_end = base_path.rfind('/').map_or(0, |index| index + 1);
        format!("{}{reference_path}", &base_path[..directory_end])
    };
    let mut result = normalize_path(&joined);
    if let Some(query) = query {
        result.push('?');
        result.push_str(query);
    }
    result
}

fn normalize_path(path: &str) -> String {
    let keep_trailing_slash = path.ends_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            value => segments.push(value),
        }
    }
    let mut normalized = format!("/{}", segments.join("/"));
    if keep_trailing_slash && normalized != "/" {
        normalized.push('/');
    }
    normalized
}

fn local_http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(0)
        .no_delay(false)
        .proxy(None)
        .build()
        .into()
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
    use super::{LoadError, local_http_agent, resolve_url_reference, validate_local_http_url};
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

    #[test]
    fn local_agent_avoids_socket_option_races() {
        let agent = local_http_agent();
        let config = agent.config();
        let timeouts = config.timeouts();

        assert!(!config.no_delay());
        assert_eq!(timeouts.global, None);
        assert_eq!(timeouts.connect, None);
        assert_eq!(timeouts.send_request, None);
        assert_eq!(timeouts.recv_response, None);
        assert_eq!(timeouts.recv_body, None);
    }

    #[test]
    fn resolves_document_urls_with_queries_and_fragments() {
        let base = "http://localhost:3000/guide/current?old=1";

        assert_eq!(
            resolve_url_reference(base, "../next?q=1#details").unwrap(),
            "http://localhost:3000/next?q=1#details"
        );
        assert_eq!(
            resolve_url_reference(base, "#details").unwrap(),
            "http://localhost:3000/guide/current?old=1#details"
        );
    }
}
