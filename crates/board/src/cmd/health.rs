//! In-process health probe used by container healthchecks.
//!
//! The runtime image is distroless and deliberately ships neither a shell nor
//! `curl`, so the container healthcheck calls back into this binary instead:
//!
//! ```text
//! /app/board health --config-file /app/config/backend.yaml
//! ```
//!
//! The probe resolves the listen address through the same configuration path as
//! the server, so a changed config or `PORT` cannot desynchronize the two.
//!
//! It speaks a single HTTP/1.1 request over `tokio::net::TcpStream` rather than
//! pulling in an HTTP client crate: the probe only needs a status line, and an
//! extra client stack would grow the runtime image it exists to keep small.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::cmd::CliArgs;
use crate::init::cfg_service;
use crate::route::API_PREFIX;

/// Upper bound for the whole probe (connect + request + response head).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 5;

/// Largest response head we are willing to buffer before giving up.
const MAX_RESPONSE_HEAD_BYTES: usize = 8 * 1024;

/// Path probed by the health command; kept next to the router that serves it.
pub(crate) fn health_path() -> String {
    format!("{API_PREFIX}/health")
}

/// Maps a configured listen address to an address that can actually be dialed.
///
/// The server usually binds a wildcard address (`0.0.0.0` / `::`), which is not
/// a valid destination. Everything else is probed as configured so that a
/// loopback-only or explicitly bound deployment still works.
pub(crate) fn dial_host(listen_addr: &str) -> &str {
    match listen_addr.trim() {
        "" | "0.0.0.0" => "127.0.0.1",
        "::" | "[::]" | "0:0:0:0:0:0:0:0" => "::1",
        other => other,
    }
}

/// Wraps an IPv6 literal in brackets so it is usable in a socket address or a
/// `Host` header.
fn bracketed(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

/// Builds the minimal HTTP/1.1 request the probe sends.
pub(crate) fn health_request(host: &str, port: u16) -> String {
    let host_header = bracketed(host);
    format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host_header}:{port}\r\n\
         User-Agent: elizabeth-health/{version}\r\n\
         Accept: */*\r\n\
         Connection: close\r\n\
         \r\n",
        path = health_path(),
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Extracts the status code from an HTTP response head.
///
/// Returns `None` when the payload is not an HTTP status line, so a truncated or
/// non-HTTP response is reported as a failure instead of being misread as ready.
pub(crate) fn parse_status_code(head: &str) -> Option<u16> {
    let status_line = head.lines().next()?.trim_end_matches('\r');
    let mut parts = status_line.split(' ');
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse().ok()
}

/// Reads until the response head is complete, or the peer closes the stream.
async fn read_response_head(stream: &mut TcpStream, address: &str) -> Result<String> {
    let mut head = Vec::with_capacity(256);
    let mut chunk = [0_u8; 256];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream
            .read(&mut chunk)
            .await
            .with_context(|| format!("failed to read the health response from {address}"))?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..read]);
        if head.len() > MAX_RESPONSE_HEAD_BYTES {
            bail!("health response from {address} has no complete header");
        }
    }
    Ok(String::from_utf8_lossy(&head).into_owned())
}

async fn probe(host: &str, port: u16, timeout: Duration) -> Result<u16> {
    let address = format!("{}:{port}", bracketed(host));
    let probe = async {
        let mut stream = TcpStream::connect(&address)
            .await
            .with_context(|| format!("failed to connect to {address}"))?;
        stream
            .write_all(health_request(host, port).as_bytes())
            .await
            .with_context(|| format!("failed to send the health request to {address}"))?;
        let head = read_response_head(&mut stream, &address).await?;
        parse_status_code(&head)
            .with_context(|| format!("unexpected health response from {address}: {}", head.trim()))
    };

    tokio::time::timeout(timeout, probe)
        .await
        .with_context(|| format!("health probe for {address} timed out after {timeout:?}"))?
}

/// Probes the local health endpoint and exits non-zero when it is not ready.
pub async fn run(common: &CliArgs) -> Result<()> {
    let cfg = cfg_service::init(common)?;
    let host = dial_host(&cfg.app.server.addr);
    let port = cfg.app.server.port;

    let status = probe(host, port, Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)).await?;
    if status != 200 {
        bail!("health endpoint on {host}:{port} returned HTTP {status}");
    }
    println!("OK");
    Ok(())
}
