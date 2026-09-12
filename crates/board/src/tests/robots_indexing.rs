use axum::body::Body;
use axum::http::{Request, StatusCode};

use crate::{robots_txt_body, serve_html_asset, spa_fallback};

fn noindex_header(response: &axum::response::Response) -> Option<&str> {
    response
        .headers()
        .get("x-robots-tag")
        .and_then(|value| value.to_str().ok())
}

async fn request_at(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("valid request")
}

#[test]
fn robots_txt_disallows_all_crawlers_when_indexing_disallowed() {
    assert_eq!(robots_txt_body(true), "User-agent: *\nDisallow: /\n");
}

#[test]
fn robots_txt_explicitly_allows_crawlers_when_indexing_allowed() {
    assert_eq!(robots_txt_body(false), "User-agent: *\nAllow: /\n");
}

#[test]
fn security_config_disallows_search_indexing_by_default() {
    assert!(configrs::SecurityConfig::default().disallow_search_indexing);
}

#[test]
fn html_fallback_carries_noindex_header_when_indexing_disallowed() {
    let response = serve_html_asset("index.html", true);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(noindex_header(&response), Some("noindex"));
}

#[test]
fn html_fallback_has_no_robots_header_when_indexing_allowed() {
    let response = serve_html_asset("index.html", false);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(noindex_header(&response), None);
}

#[tokio::test]
async fn spa_root_fallback_marks_html_noindex_when_indexing_disallowed() {
    let response = spa_fallback(request_at("/").await, true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(noindex_header(&response), Some("noindex"));
}

#[tokio::test]
async fn spa_room_fallback_stays_indexable_when_indexing_allowed() {
    let response = spa_fallback(request_at("/some-room").await, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(noindex_header(&response), None);
}

#[tokio::test]
async fn api_paths_never_fall_through_to_html_regardless_of_indexing_policy() {
    for disallow in [true, false] {
        let response = spa_fallback(request_at("/api/v1/missing").await, disallow).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(noindex_header(&response), None);
    }
}
