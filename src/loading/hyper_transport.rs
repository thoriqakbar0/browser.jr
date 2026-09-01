use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use rustls::{ClientConfig, RootCertStore};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use url::Url;

use super::{
    LoadError, MAX_HTML_BYTES, MAX_RESPONSE_HEADER_BYTES, MAX_RESPONSE_HEADERS, NetworkResponse,
    NetworkTransport, is_redirect_status,
};

pub(super) struct HyperNetworkTransport {
    runtime: tokio::runtime::Runtime,
}

impl HyperNetworkTransport {
    pub(super) fn new() -> Result<Self, LoadError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| LoadError::Request(error.to_string()))?;
        Ok(Self { runtime })
    }
}

impl NetworkTransport for HyperNetworkTransport {
    fn send_get(
        &self,
        url: &Url,
        approved_endpoints: &[SocketAddr],
        timeout: Duration,
    ) -> Result<NetworkResponse, LoadError> {
        self.runtime.block_on(async {
            tokio::time::timeout(timeout, send_hyper_request(url, approved_endpoints))
                .await
                .map_err(|_| LoadError::Request("request timed out".into()))?
        })
    }
}

async fn send_hyper_request(
    url: &Url,
    approved_endpoints: &[SocketAddr],
) -> Result<NetworkResponse, LoadError> {
    let mut last_error = None;
    for endpoint in approved_endpoints {
        match send_hyper_request_to_endpoint(url, *endpoint).await {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| LoadError::Request("no approved endpoints".into())))
}

async fn send_hyper_request_to_endpoint(
    url: &Url,
    endpoint: SocketAddr,
) -> Result<NetworkResponse, LoadError> {
    let stream = TcpStream::connect(endpoint)
        .await
        .map_err(|error| LoadError::Request(error.to_string()))?;
    let peer = stream
        .peer_addr()
        .map_err(|error| LoadError::Request(error.to_string()))?;
    if peer != endpoint {
        return Err(LoadError::Request(format!(
            "connected peer {peer} does not match approved endpoint {endpoint}"
        )));
    }

    if url.scheme() == "https" {
        let host = url
            .host_str()
            .ok_or_else(|| LoadError::InvalidUrl("the URL has no host".into()))?;
        let server_name = rustls::pki_types::ServerName::try_from(host.to_owned())
            .map_err(|error| LoadError::Request(error.to_string()))?;
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let tls = TlsConnector::from(Arc::new(config))
            .connect(server_name, stream)
            .await
            .map_err(|error| LoadError::Request(error.to_string()))?;
        send_hyper_request_over_io(url, tls).await
    } else {
        send_hyper_request_over_io(url, stream).await
    }
}

async fn send_hyper_request_over_io<IO>(url: &Url, io: IO) -> Result<NetworkResponse, LoadError>
where
    IO: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) = http1::Builder::new()
        .max_headers(MAX_RESPONSE_HEADERS)
        .max_buf_size(MAX_RESPONSE_HEADER_BYTES)
        .handshake(TokioIo::new(io))
        .await
        .map_err(|error| LoadError::Request(error.to_string()))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let authority = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
        None => url.host_str().unwrap_or_default().to_owned(),
    };
    let path = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };
    let request = Request::builder()
        .method("GET")
        .uri(if path.is_empty() { "/" } else { &path })
        .header("host", authority)
        .header("connection", "close")
        .body(Empty::<Bytes>::new())
        .map_err(|error| LoadError::Request(error.to_string()))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|error| LoadError::Request(error.to_string()))?;
    let status = response.status().as_u16();
    let redirect_location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if let Some(content_length) = response.headers().get("content-length") {
        let content_length = content_length
            .to_str()
            .map_err(|error| LoadError::InvalidBody(error.to_string()))?
            .parse::<u64>()
            .map_err(|error| LoadError::InvalidBody(error.to_string()))?;
        if content_length > MAX_HTML_BYTES {
            return Err(LoadError::InvalidBody(format!(
                "response exceeds {MAX_HTML_BYTES} bytes"
            )));
        }
    }
    if response
        .headers()
        .get("content-encoding")
        .is_some_and(|value| value.as_bytes() != b"identity")
    {
        return Err(LoadError::InvalidBody(
            "compressed response bodies are not supported".into(),
        ));
    }
    let body = if is_redirect_status(status) {
        None
    } else {
        let mut incoming = response.into_body();
        let mut bytes = Vec::new();
        while let Some(frame) = incoming.frame().await {
            let frame = frame.map_err(|error| LoadError::InvalidBody(error.to_string()))?;
            if let Ok(data) = frame.into_data() {
                let next_len = bytes.len().saturating_add(data.len());
                if next_len as u64 > MAX_HTML_BYTES {
                    return Err(LoadError::InvalidBody(format!(
                        "response exceeds {MAX_HTML_BYTES} bytes"
                    )));
                }
                bytes.extend_from_slice(&data);
            }
        }
        Some(String::from_utf8(bytes).map_err(|error| LoadError::InvalidBody(error.to_string()))?)
    };
    Ok(NetworkResponse {
        status,
        redirect_location,
        content_type,
        body,
    })
}
