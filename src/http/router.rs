use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Request as AxumRequest, State},
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};

use crate::{
    app::state::AppState,
    http::handlers::{
        contact_cv, curriculum, curriculum_pdf, homepage, languages, manifest, menu, not_found,
        robots, security_txt, sitemap,
    },
};

const CONTENT_SECURITY_POLICY_HTTPS: &str = "default-src 'self'; base-uri 'self'; connect-src 'self'; font-src 'self'; form-action 'self'; frame-ancestors 'none'; img-src 'self' data:; manifest-src 'self'; object-src 'none'; script-src 'self'; style-src 'self'; upgrade-insecure-requests";
const CONTENT_SECURITY_POLICY_HTTP: &str = "default-src 'self'; base-uri 'self'; connect-src 'self'; font-src 'self'; form-action 'self'; frame-ancestors 'none'; img-src 'self' data:; manifest-src 'self'; object-src 'none'; script-src 'self'; style-src 'self'";
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const PERMISSIONS_POLICY: &str = "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()";
const REFERRER_POLICY: &str = "no-referrer";
const CONTACT_BODY_LIMIT: usize = 32 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub fn build_router(state: AppState) -> Router {
    let assets_dir = ServeDir::new("assets").precompressed_gzip();
    let trusted_proxies = state.trusted_proxies.clone();

    let assets_service = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=604800, immutable"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::VARY,
            HeaderValue::from_static("accept-encoding"),
        ))
        .service(assets_dir);

    let router = Router::new()
        .route("/", get(homepage))
        .route("/{lang}", get(homepage))
        .route("/{lang}/", get(homepage))
        .route("/curriculum", get(curriculum))
        .route("/curriculum/", get(curriculum))
        .route("/curriculum.pdf", get(curriculum_pdf))
        .route("/{lang}/curriculum", get(curriculum))
        .route("/{lang}/curriculum.pdf", get(curriculum_pdf))
        .route("/{lang}/curriculum/", get(curriculum))
        .route("/menu", get(menu))
        .route("/menu/", get(menu))
        .route("/{lang}/menu", get(menu))
        .route("/{lang}/menu/", get(menu))
        .route("/languages", get(languages))
        .route("/languages/", get(languages))
        .route("/{lang}/languages", get(languages))
        .route("/{lang}/languages/", get(languages))
        .route("/contact", post(contact_cv))
        .route("/{lang}/contact", post(contact_cv))
        .route("/{lang}/contact/", post(contact_cv))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        .route("/site.webmanifest", get(manifest))
        .route("/.well-known/security.txt", get(security_txt))
        .nest_service("/assets", assets_service)
        .fallback(not_found)
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(trusted_proxies, security_headers_middleware))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("permissions-policy"),
                    HeaderValue::from_static(PERMISSIONS_POLICY),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("referrer-policy"),
                    HeaderValue::from_static(REFERRER_POLICY),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("x-content-type-options"),
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("x-frame-options"),
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("cross-origin-opener-policy"),
                    HeaderValue::from_static("same-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("cross-origin-resource-policy"),
                    HeaderValue::from_static("same-origin"),
                ))
                .layer(SetResponseHeaderLayer::if_not_present(
                    CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=300"),
                )),
        )
        .with_state(state);

    router
        .layer(DefaultBodyLimit::max(CONTACT_BODY_LIMIT))
        .layer(middleware::from_fn(timeout_middleware))
        .layer(CompressionLayer::new())
}

async fn security_headers_middleware(
    State(trusted_proxies): State<Arc<[IpAddr]>>,
    request: AxumRequest<Body>,
    next: Next,
) -> Response {
    let is_https = request_is_https(&request, &trusted_proxies);
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    let csp = if is_https { CONTENT_SECURITY_POLICY_HTTPS } else { CONTENT_SECURITY_POLICY_HTTP };
    if let Ok(value) = HeaderValue::from_str(csp) {
        headers.insert(HeaderName::from_static("content-security-policy"), value);
    }

    if is_https && let Ok(value) = HeaderValue::from_str(HSTS) {
        headers.insert(HeaderName::from_static("strict-transport-security"), value);
    }

    response
}

fn request_is_https(request: &AxumRequest<Body>, trusted_proxies: &[IpAddr]) -> bool {
    let peer =
        request.extensions().get::<ConnectInfo<SocketAddr>>().map(|ConnectInfo(addr)| addr.ip());
    let forwarded_proto =
        request.headers().get("x-forwarded-proto").and_then(|value| value.to_str().ok());

    request_is_https_from_parts(request.uri().scheme_str(), peer, forwarded_proto, trusted_proxies)
}

fn request_is_https_from_parts(
    scheme: Option<&str>,
    peer: Option<IpAddr>,
    forwarded_proto: Option<&str>,
    trusted_proxies: &[IpAddr],
) -> bool {
    if scheme == Some("https") {
        return true;
    }

    // Trust loopback by default (local reverse proxy). Optional TRUSTED_PROXIES for remote proxies.
    let peer_is_trusted = peer.is_some_and(|ip| ip.is_loopback() || trusted_proxies.contains(&ip));
    if !peer_is_trusted {
        return false;
    }

    forwarded_proto.is_some_and(|value| {
        value.split(',').next().is_some_and(|part| part.trim().eq_ignore_ascii_case("https"))
    })
}

async fn timeout_middleware(request: AxumRequest<Body>, next: Next) -> Response {
    match timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), next.run(request)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::REQUEST_TIMEOUT, "Request timed out").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::request_is_https_from_parts;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn https_scheme_is_trusted_without_forwarded_headers() {
        assert!(request_is_https_from_parts(Some("https"), None, None, &[]));
        assert!(request_is_https_from_parts(
            Some("https"),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))),
            None,
            &[]
        ));
    }

    #[test]
    fn forwarded_https_is_trusted_only_from_loopback_by_default() {
        assert!(request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            Some("https"),
            &[]
        ));
        assert!(request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            Some("https, http"),
            &[]
        ));
        assert!(!request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))),
            Some("https"),
            &[]
        ));
        assert!(!request_is_https_from_parts(Some("http"), None, Some("https"), &[]));
    }

    #[test]
    fn forwarded_https_is_trusted_from_configured_proxies() {
        let trusted = [IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))];
        assert!(request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))),
            Some("https"),
            &trusted
        ));
        assert!(!request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6))),
            Some("https"),
            &trusted
        ));
    }

    #[test]
    fn non_https_forwarded_proto_is_ignored() {
        assert!(!request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            Some("http"),
            &[]
        ));
        assert!(!request_is_https_from_parts(
            Some("http"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            None,
            &[]
        ));
    }
}
