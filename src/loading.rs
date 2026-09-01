use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use http::Uri;
use url::{Host, Url};

mod hyper_transport;

use hyper_transport::HyperNetworkTransport;

const MAX_HTML_BYTES: u64 = 1024 * 1024;
const MAX_REDIRECTS: u32 = 5;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_DNS_ANSWERS: usize = 16;
const MAX_RESPONSE_HEADERS: usize = 64;
const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoadedHtml {
    pub(crate) final_url: String,
    pub(crate) html: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidUrl(String),
    UnsupportedTarget(String),
    BlockedAddress(IpAddr),
    RedirectDowngrade,
    TooManyRedirects(u32),
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
            Self::BlockedAddress(address) => {
                write!(formatter, "blocked network address: {address}")
            }
            Self::RedirectDowngrade => {
                write!(formatter, "HTTPS redirects cannot downgrade to HTTP")
            }
            Self::TooManyRedirects(limit) => {
                write!(formatter, "redirect limit exceeded ({limit})")
            }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NetworkMode {
    PublicOnly,
    LoopbackOnly,
}

trait NetworkResolver {
    fn resolve_url(&self, url: &Url, timeout: Duration) -> Result<Vec<SocketAddr>, LoadError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NetworkResponse {
    status: u16,
    redirect_location: Option<String>,
    content_type: Option<String>,
    body: Option<String>,
}

trait NetworkTransport {
    fn send_get(
        &self,
        url: &Url,
        approved_endpoints: &[SocketAddr],
        timeout: Duration,
    ) -> Result<NetworkResponse, LoadError>;
}

struct FetchEngine<R, T> {
    resolver: R,
    transport: T,
    mode: NetworkMode,
}

impl<R: NetworkResolver, T: NetworkTransport> FetchEngine<R, T> {
    fn new(resolver: R, transport: T, mode: NetworkMode) -> Self {
        Self {
            resolver,
            transport,
            mode,
        }
    }

    fn fetch(&self, value: &str) -> Result<LoadedHtml, LoadError> {
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let mut current_url = parse_network_url(value)?;
        let mut visited = Vec::new();

        for redirect_count in 0..=MAX_REDIRECTS {
            reject_redirect_loop(&visited, &current_url)?;
            visited.push(current_url.clone());
            let endpoints = self.resolve_approved_endpoints(&current_url, deadline)?;
            let response = self.send_with_deadline(&current_url, &endpoints, deadline)?;
            match process_network_response(&current_url, response, redirect_count)? {
                FetchStep::Redirect(next_url) => current_url = next_url,
                FetchStep::Complete(loaded) => return Ok(loaded),
            }
        }

        Err(LoadError::TooManyRedirects(MAX_REDIRECTS))
    }

    fn resolve_approved_endpoints(
        &self,
        url: &Url,
        deadline: Instant,
    ) -> Result<Vec<SocketAddr>, LoadError> {
        let remaining = remaining_request_time(deadline)?;
        let endpoints = self.resolver.resolve_url(url, remaining)?;
        if endpoints.is_empty() {
            return Err(LoadError::Request("DNS returned no addresses".into()));
        }
        for endpoint in &endpoints {
            if !address_is_allowed_for_mode(endpoint.ip(), self.mode) {
                return Err(LoadError::BlockedAddress(endpoint.ip()));
            }
        }
        Ok(endpoints)
    }

    fn send_with_deadline(
        &self,
        url: &Url,
        endpoints: &[SocketAddr],
        deadline: Instant,
    ) -> Result<NetworkResponse, LoadError> {
        self.transport
            .send_get(url, endpoints, remaining_request_time(deadline)?)
    }

    #[cfg(test)]
    fn transport(&self) -> &T {
        &self.transport
    }
}

#[derive(Debug, Eq, PartialEq)]
enum FetchStep {
    Redirect(Url),
    Complete(LoadedHtml),
}

fn remaining_request_time(deadline: Instant) -> Result<Duration, LoadError> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| LoadError::Request("request timed out".into()))
}

fn reject_redirect_loop(visited: &[Url], url: &Url) -> Result<(), LoadError> {
    if visited.contains(url) {
        Err(LoadError::TooManyRedirects(MAX_REDIRECTS))
    } else {
        Ok(())
    }
}

fn process_network_response(
    current_url: &Url,
    response: NetworkResponse,
    redirect_count: u32,
) -> Result<FetchStep, LoadError> {
    if is_redirect_status(response.status) {
        return next_redirect_url(current_url, response.redirect_location, redirect_count)
            .map(FetchStep::Redirect);
    }
    if !(200..300).contains(&response.status) {
        return Err(LoadError::UnexpectedStatus(response.status));
    }
    validate_html_content_type(response.content_type.as_deref())?;
    let html = response
        .body
        .ok_or_else(|| LoadError::InvalidBody("response body is missing".into()))?;
    Ok(FetchStep::Complete(LoadedHtml {
        final_url: current_url.to_string(),
        html,
    }))
}

fn next_redirect_url(
    current_url: &Url,
    location: Option<String>,
    redirect_count: u32,
) -> Result<Url, LoadError> {
    if redirect_count == MAX_REDIRECTS {
        return Err(LoadError::TooManyRedirects(MAX_REDIRECTS));
    }
    let location = location
        .ok_or_else(|| LoadError::Request("redirect response has no Location header".into()))?;
    let next_url = current_url
        .join(&location)
        .map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    validate_parsed_network_url(&next_url)?;
    if current_url.scheme() == "https" && next_url.scheme() == "http" {
        return Err(LoadError::RedirectDowngrade);
    }
    Ok(next_url)
}

fn address_is_allowed_for_mode(address: IpAddr, mode: NetworkMode) -> bool {
    match mode {
        NetworkMode::PublicOnly => is_permitted_address(address, false),
        NetworkMode::LoopbackOnly => address.is_loopback(),
    }
}

pub(crate) fn load_html(value: &str) -> Result<LoadedHtml, LoadError> {
    let url = parse_network_url(value)?;
    let mode = network_mode_for_url(&url);
    FetchEngine::new(SystemNetworkResolver, HyperNetworkTransport::new()?, mode).fetch(value)
}

fn parse_network_url(value: &str) -> Result<Url, LoadError> {
    let mut url = Url::parse(value).map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    validate_parsed_network_url(&url)?;
    url.set_fragment(None);
    Ok(url)
}

fn validate_parsed_network_url(url: &Url) -> Result<(), LoadError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(LoadError::UnsupportedTarget(
            "only HTTP and HTTPS pages are supported".into(),
        ));
    }
    if url.host_str().is_none() {
        return Err(LoadError::InvalidUrl("the URL has no host".into()));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(LoadError::UnsupportedTarget(
            "credentials are not accepted".into(),
        ));
    }
    if let Some(address) = url_literal_ip(url)
        && !(is_permitted_address(address, false) || address.is_loopback())
    {
        return Err(LoadError::UnsupportedTarget(
            "private and non-routable network targets are not accepted".into(),
        ));
    }
    Ok(())
}

fn url_literal_ip(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        Host::Ipv4(address) => Some(IpAddr::V4(address)),
        Host::Ipv6(address) => Some(IpAddr::V6(address)),
        Host::Domain(_) => None,
    }
}

fn network_mode_for_url(url: &Url) -> NetworkMode {
    let is_loopback = url
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case("localhost"))
        || url_literal_ip(url).is_some_and(|address| address.is_loopback());
    if is_loopback {
        NetworkMode::LoopbackOnly
    } else {
        NetworkMode::PublicOnly
    }
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn validate_html_content_type(content_type: Option<&str>) -> Result<(), LoadError> {
    let Some(content_type) = content_type else {
        return Ok(());
    };
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if media_type != "text/html" && media_type != "application/xhtml+xml" {
        return Err(LoadError::UnsupportedContentType(content_type.into()));
    }
    Ok(())
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

#[derive(Clone, Copy, Debug)]
struct SystemNetworkResolver;

impl NetworkResolver for SystemNetworkResolver {
    fn resolve_url(&self, url: &Url, timeout: Duration) -> Result<Vec<SocketAddr>, LoadError> {
        let host = url
            .host_str()
            .ok_or_else(|| LoadError::InvalidUrl("the URL has no host".into()))?
            .to_owned();
        let port = url
            .port_or_known_default()
            .ok_or_else(|| LoadError::InvalidUrl("the URL has no port".into()))?;
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = (host.as_str(), port)
                .to_socket_addrs()
                .map(|addresses| addresses.take(MAX_DNS_ANSWERS).collect::<Vec<_>>());
            let _ = sender.send(result);
        });
        receiver
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => LoadError::Request("DNS timed out".into()),
                mpsc::RecvTimeoutError::Disconnected => {
                    LoadError::Request("DNS resolver stopped".into())
                }
            })?
            .map_err(|error| LoadError::Request(error.to_string()))
    }
}

fn is_permitted_address(address: IpAddr, allow_loopback: bool) -> bool {
    match address {
        IpAddr::V4(address) => is_permitted_ipv4(address, allow_loopback),
        IpAddr::V6(address) => {
            if let Some(address) = address.to_ipv4_mapped() {
                return is_permitted_ipv4(address, allow_loopback);
            }
            is_permitted_ipv6(address, allow_loopback)
        }
    }
}

fn is_permitted_ipv4(address: Ipv4Addr, allow_loopback: bool) -> bool {
    if allow_loopback && address.is_loopback() {
        return true;
    }
    let [first, second, third, _] = address.octets();
    !(address.is_unspecified()
        || address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || first == 0
        || (first == 100 && (64..=127).contains(&second))
        || (first == 192 && second == 0 && third == 0)
        || (first == 192 && second == 0 && third == 2)
        || (first == 198 && matches!(second, 18 | 19))
        || (first == 198 && second == 51 && third == 100)
        || (first == 203 && second == 0 && third == 113)
        || first >= 240)
}

fn is_permitted_ipv6(address: Ipv6Addr, allow_loopback: bool) -> bool {
    if allow_loopback && address.is_loopback() {
        return true;
    }
    let segments = address.segments();
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::{
        FetchEngine, HyperNetworkTransport, LoadError, MAX_HTML_BYTES, NetworkMode,
        NetworkResolver, NetworkResponse, NetworkTransport, REQUEST_TIMEOUT, is_permitted_address,
        parse_network_url, resolve_url_reference,
    };
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::time::Duration;
    use url::Url;

    struct FakeResolver {
        endpoints_by_host: HashMap<String, Vec<SocketAddr>>,
    }

    impl FakeResolver {
        fn new<const N: usize>(entries: [(&str, Vec<&str>); N]) -> Self {
            let endpoints_by_host = entries
                .into_iter()
                .map(|(host, endpoints)| {
                    (
                        host.to_owned(),
                        endpoints
                            .into_iter()
                            .map(|endpoint| endpoint.parse().unwrap())
                            .collect(),
                    )
                })
                .collect();
            Self { endpoints_by_host }
        }
    }

    impl NetworkResolver for FakeResolver {
        fn resolve_url(&self, url: &Url, _timeout: Duration) -> Result<Vec<SocketAddr>, LoadError> {
            self.endpoints_by_host
                .get(url.host_str().unwrap_or_default())
                .cloned()
                .ok_or_else(|| LoadError::Request("fake DNS answer missing".into()))
        }
    }

    struct FakeTransport {
        source_url: String,
        redirect_location: String,
        request_count: Cell<usize>,
    }

    impl FakeTransport {
        fn redirect(source_url: &str, redirect_location: &str) -> Self {
            Self {
                source_url: source_url.into(),
                redirect_location: redirect_location.into(),
                request_count: Cell::new(0),
            }
        }

        fn request_count(&self) -> usize {
            self.request_count.get()
        }
    }

    impl NetworkTransport for FakeTransport {
        fn send_get(
            &self,
            url: &Url,
            _approved_endpoints: &[SocketAddr],
            _timeout: Duration,
        ) -> Result<NetworkResponse, LoadError> {
            self.request_count.set(self.request_count.get() + 1);
            if url.as_str() == self.source_url {
                Ok(NetworkResponse {
                    status: 302,
                    redirect_location: Some(self.redirect_location.clone()),
                    content_type: None,
                    body: None,
                })
            } else {
                Ok(NetworkResponse {
                    status: 200,
                    redirect_location: None,
                    content_type: Some("text/html".into()),
                    body: Some("<p>ok</p>".into()),
                })
            }
        }
    }

    #[test]
    fn accepts_public_and_loopback_http_urls() {
        for value in [
            "http://example.com/page",
            "https://example.com/page",
            "http://localhost:3000/page",
            "http://127.0.0.1:3000/page",
            "http://[::1]:3000/page",
        ] {
            assert!(parse_network_url(value).is_ok());
        }
    }

    #[test]
    fn rejects_credentials_and_unsupported_schemes() {
        for value in ["ftp://example.com/page", "https://user@example.com/page"] {
            let result = parse_network_url(value);
            assert!(matches!(result, Err(LoadError::UnsupportedTarget(_))));
        }
    }

    #[test]
    fn rejects_literal_private_and_non_routable_targets() {
        for value in [
            "http://10.0.0.1/page",
            "http://169.254.1.1/page",
            "http://192.168.1.1/page",
            "http://[fc00::1]/page",
            "http://[fe80::1]/page",
        ] {
            let result = parse_network_url(value);
            assert!(
                matches!(result, Err(LoadError::UnsupportedTarget(_))),
                "{value}: {result:?}"
            );
        }
    }

    #[test]
    fn filters_private_and_reserved_resolved_addresses() {
        for value in [
            "10.0.0.1",
            "100.64.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.1.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
        ] {
            assert!(!is_permitted_address(
                value.parse::<IpAddr>().unwrap(),
                false
            ));
        }
        for value in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_permitted_address(
                value.parse::<IpAddr>().unwrap(),
                false
            ));
        }
        assert!(is_permitted_address("127.0.0.1".parse().unwrap(), true));
        assert!(!is_permitted_address("127.0.0.1".parse().unwrap(), false));
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

    #[test]
    fn redirect_to_private_address_fails_before_second_request() {
        let resolver = FakeResolver::new([
            ("public.example", vec!["93.184.216.34:443"]),
            ("private.example", vec!["10.0.0.1:443"]),
        ]);
        let transport =
            FakeTransport::redirect("https://public.example/", "https://private.example/secret");
        let engine = FetchEngine::new(resolver, transport, NetworkMode::PublicOnly);

        let result = engine.fetch("https://public.example/");

        assert!(matches!(result, Err(LoadError::BlockedAddress(_))));
        assert_eq!(engine.transport().request_count(), 1);
    }

    #[test]
    fn https_redirect_downgrade_fails_before_second_request() {
        let resolver = FakeResolver::new([
            ("secure.example", vec!["93.184.216.34:443"]),
            ("plain.example", vec!["93.184.216.35:80"]),
        ]);
        let transport = FakeTransport::redirect("https://secure.example/", "http://plain.example/");
        let engine = FetchEngine::new(resolver, transport, NetworkMode::PublicOnly);

        let result = engine.fetch("https://secure.example/");

        assert_eq!(result, Err(LoadError::RedirectDowngrade));
        assert_eq!(engine.transport().request_count(), 1);
    }

    #[test]
    fn hyper_transport_uses_the_approved_endpoint() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, peer) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" || header.is_empty() {
                    break;
                }
            }
            let body = "<p>approved</p>";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            (peer.ip(), request_line)
        });
        let url = Url::parse("http://public.example/test").unwrap();

        let response = HyperNetworkTransport::new()
            .unwrap()
            .send_get(&url, &[endpoint], REQUEST_TIMEOUT)
            .unwrap();
        let (peer, request_line) = server.join().unwrap();

        assert!(peer.is_loopback());
        assert_eq!(request_line, "GET /test HTTP/1.1\r\n");
        assert_eq!(response.body.as_deref(), Some("<p>approved</p>"));
    }

    #[test]
    fn hyper_transport_rejects_oversized_content_length_before_body_read() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                MAX_HTML_BYTES + 1
            )
            .unwrap();
        });
        let url = Url::parse("http://public.example/").unwrap();

        let result =
            HyperNetworkTransport::new()
                .unwrap()
                .send_get(&url, &[endpoint], REQUEST_TIMEOUT);
        server.join().unwrap();

        assert!(matches!(
            result,
            Err(LoadError::InvalidBody(reason)) if reason.contains("exceeds")
        ));
    }
}
