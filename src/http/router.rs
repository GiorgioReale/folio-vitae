use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Request as AxumRequest},
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::time::Duration;
use tokio::time::timeout;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};

use crate::{
    app::state::AppState,
    http::handlers::{
        contact_cv, curriculum, homepage, manifest, menu, not_found, robots, security_txt, sitemap,
    },
};

const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; base-uri 'self'; connect-src 'self'; font-src 'self'; form-action 'self'; frame-ancestors 'none'; img-src 'self' data:; manifest-src 'self'; object-src 'none'; script-src 'self'; style-src 'self'; upgrade-insecure-requests";
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const PERMISSIONS_POLICY: &str = "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()";
const REFERRER_POLICY: &str = "no-referrer";
const CONTACT_BODY_LIMIT: usize = 32 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 10;

pub fn build_router(state: AppState) -> Router {
    let assets_dir = ServeDir::new("assets").precompressed_gzip();

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
        .route("/{lang}/curriculum", get(curriculum))
        .route("/{lang}/curriculum/", get(curriculum))
        .route("/menu", get(menu))
        .route("/{lang}/menu", get(menu))
        .route("/{lang}/menu/", get(menu))
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
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("content-security-policy"),
                    HeaderValue::from_static(CONTENT_SECURITY_POLICY),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("strict-transport-security"),
                    HeaderValue::from_static(HSTS),
                ))
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

async fn timeout_middleware(request: AxumRequest<Body>, next: Next) -> Response {
    match timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), next.run(request)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::REQUEST_TIMEOUT, "Request timed out").into_response(),
    }
}
