use std::time::Duration;

use crate::cmd::health;
use crate::route::API_PREFIX;

#[test]
fn health_path_follows_the_api_prefix() {
    assert_eq!(health::health_path(), format!("{API_PREFIX}/health"));
}

#[test]
fn wildcard_listen_addresses_are_probed_on_loopback() {
    assert_eq!(health::dial_host("0.0.0.0"), "127.0.0.1");
    assert_eq!(health::dial_host(""), "127.0.0.1");
    assert_eq!(health::dial_host(" 0.0.0.0 "), "127.0.0.1");
    assert_eq!(health::dial_host("::"), "::1");
    assert_eq!(health::dial_host("[::]"), "::1");
}

#[test]
fn explicit_listen_addresses_are_probed_as_configured() {
    assert_eq!(health::dial_host("127.0.0.1"), "127.0.0.1");
    assert_eq!(health::dial_host("10.0.0.5"), "10.0.0.5");
    assert_eq!(health::dial_host("::1"), "::1");
}

#[test]
fn request_targets_the_health_endpoint_with_a_host_header() {
    let request = health::health_request("127.0.0.1", 4092);
    assert!(
        request.starts_with(&format!("GET {} HTTP/1.1\r\n", health::health_path())),
        "unexpected request line: {request:?}"
    );
    assert!(request.contains("Host: 127.0.0.1:4092\r\n"), "{request:?}");
    assert!(request.ends_with("\r\n\r\n"), "{request:?}");
}

#[test]
fn ipv6_hosts_are_bracketed_in_the_host_header() {
    let request = health::health_request("::1", 4092);
    assert!(request.contains("Host: [::1]:4092\r\n"), "{request:?}");
}

#[test]
fn status_codes_are_parsed_from_the_response_head() {
    assert_eq!(health::parse_status_code("HTTP/1.1 200 OK\r\n"), Some(200));
    assert_eq!(
        health::parse_status_code("HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n"),
        Some(503)
    );
    assert_eq!(
        health::parse_status_code("HTTP/1.0 204 No Content\n"),
        Some(204)
    );
}

#[test]
fn non_http_responses_are_rejected_instead_of_being_read_as_ready() {
    assert_eq!(health::parse_status_code(""), None);
    assert_eq!(health::parse_status_code("\r\n"), None);
    assert_eq!(health::parse_status_code("OK"), None);
    assert_eq!(health::parse_status_code("HTTP/1.1 abc OK\r\n"), None);
}

#[test]
fn probe_timeout_stays_within_the_compose_healthcheck_budget() {
    // docker-compose.yml sets `timeout: 10s`; the probe must give up sooner so the
    // failure is reported by this process instead of being killed by the runtime.
    assert!(Duration::from_secs(health::DEFAULT_TIMEOUT_SECONDS) < Duration::from_secs(10));
}
