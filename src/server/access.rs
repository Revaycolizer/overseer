//! Loopback is not a browser trust boundary: reject cross-site requests and
//! DNS rebinding before they can reach local tools or credential storage.
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode, Method},
    middleware::Next,
    response::{Response, IntoResponse},
};

fn trusted_authority(value: &str) -> bool {
    let host = value.split(':').next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "tauri.localhost")
}

fn trusted_origin(value: &str) -> bool {
    if value == "tauri://localhost" {
        return true;
    }
    let Some((scheme, authority)) = value.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && !authority.contains(['/', '@', '?', '#'])
        && trusted_authority(authority)
}

fn check(headers: &HeaderMap) -> Result<(), StatusCode> {
    // Browsers omit Origin on some cross-site GETs (images and navigation).
    if !headers.contains_key("origin")
        && headers.get("sec-fetch-site").and_then(|v| v.to_str().ok()) == Some("cross-site")
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(host) = headers.get("host") {
        if !host.to_str().map(trusted_authority).unwrap_or(false) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    if let Some(origin) = headers.get("origin") {
        if !origin.to_str().map(trusted_origin).unwrap_or(false) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(())
}

pub async fn guard(request: Request, next: Next) -> Result<Response, StatusCode> {
    check(request.headers())?;
    let origin = request.headers().get("origin").cloned();
    // Native webviews and the local development UI need authenticated REST
    // preflights. Origins have already passed the loopback trust check above.
    if request.method() == Method::OPTIONS && origin.is_some() {
        let allowed = request.headers().get("access-control-request-headers")
            .and_then(|v| v.to_str().ok()).unwrap_or("").split(',')
            .all(|h| matches!(h.trim().to_ascii_lowercase().as_str(), "" | "authorization" | "content-type"));
        if !allowed { return Err(StatusCode::FORBIDDEN); }
        let mut response = StatusCode::NO_CONTENT.into_response();
        response.headers_mut().insert("access-control-allow-origin", origin.unwrap());
        response.headers_mut().insert("access-control-allow-methods", "GET, POST, PATCH, DELETE, OPTIONS".parse().unwrap());
        response.headers_mut().insert("access-control-allow-headers", "Authorization, Content-Type".parse().unwrap());
        response.headers_mut().insert("vary", "Origin".parse().unwrap());
        return Ok(response);
    }
    let path = request.uri().path();
    // Chat authenticates using its first WebSocket frame because browser sockets
    // cannot attach Authorization headers. Health remains available to probes.
    if !matches!(path, "/api/ws/chat" | "/health" | "/healthz") {
        if let Ok(token) = std::env::var("OVERSEER_API_TOKEN") {
            if !token.is_empty() {
                let expected = format!("Bearer {token}");
                if request
                    .headers()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    != Some(expected.as_str())
                {
                    let mut response = StatusCode::UNAUTHORIZED.into_response();
                    if let Some(origin) = origin {
                        response.headers_mut().insert("access-control-allow-origin", origin);
                        response.headers_mut().insert("vary", "Origin".parse().unwrap());
                    }
                    return Ok(response);
                }
            }
        }
    }
    let mut response = next.run(request).await;
    if let Some(origin) = origin {
        response.headers_mut().insert("access-control-allow-origin", origin);
        response.headers_mut().insert("vary", "Origin".parse().unwrap());
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_websites_and_rebinding_but_allows_native_clients() {
        let mut headers = HeaderMap::new();
        assert!(check(&headers).is_ok());
        headers.insert("host", "127.0.0.1:8080".parse().unwrap());
        for origin in [
            "http://localhost:1420",
            "https://tauri.localhost",
            "tauri://localhost",
        ] {
            headers.insert("origin", origin.parse().unwrap());
            assert!(check(&headers).is_ok());
        }
        for origin in [
            "https://evil.example",
            "null",
            "http://localhost.evil.example",
            "http://localhost@evil.example",
        ] {
            headers.insert("origin", origin.parse().unwrap());
            assert_eq!(check(&headers), Err(StatusCode::FORBIDDEN));
        }
        headers.remove("origin");
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        assert_eq!(check(&headers), Err(StatusCode::FORBIDDEN));
        headers.remove("sec-fetch-site");
        headers.insert("host", "evil.example:8080".parse().unwrap());
        assert_eq!(check(&headers), Err(StatusCode::FORBIDDEN));
    }
}
