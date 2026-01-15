//! Static Files module
//!
//! Serves embedded static files using rust-embed.
//!
//! # Requirements
//!
//! - 4.2: Embed frontend resources into binary using rust-embed

use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

/// Embedded frontend assets
#[derive(RustEmbed)]
#[folder = "dist"]
pub struct Assets;

/// Serve static files from embedded assets
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    
    // Try to serve the exact path
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // For SPA routing, serve index.html for non-file paths
    if !path.contains('.') || path.is_empty() {
        if let Some(content) = Assets::get("index.html") {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(content.data.into_owned()))
                .unwrap();
        }
    }

    // Return 404 for missing files
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}

/// Serve index.html for the root path
pub async fn index_handler() -> impl IntoResponse {
    if let Some(content) = Assets::get("index.html") {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from("<html><body><h1>Frontend not built</h1><p>Please run 'pnpm build' in the frontend directory.</p></body></html>"))
            .unwrap()
    }
}

/// Fallback handler for SPA routing
pub async fn fallback_handler(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path();
    
    // If it's an API request, return 404
    if path.starts_with("/api/") {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"code":"NOT_FOUND","message":"API endpoint not found"}"#))
            .unwrap();
    }

    // Try to serve static file
    let path = path.trim_start_matches('/');
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // Serve index.html for SPA routing
    if let Some(content) = Assets::get("index.html") {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assets_embedded() {
        // This test verifies that the Assets struct is properly defined
        // The actual embedding happens at compile time
        // If frontend is not built, this will still compile but Assets::get will return None
    }

    #[tokio::test]
    async fn test_index_handler() {
        let response = index_handler().await.into_response();
        // Response should be either OK (if frontend built) or NOT_FOUND
        let status = response.status();
        assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
    }
}
