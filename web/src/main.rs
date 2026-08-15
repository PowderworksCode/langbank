//! langbank.dev — the data langbank carries, served from the statics it
//! compiles into.
//!
//! There is no database, no cache and no data directory. Every page is rendered
//! from `&'static` tables that `build.rs` produced at compile time, which is the
//! same thing any other consumer of the crate gets. If a page here is wrong, the
//! data is wrong.

use axum::{
    Form, Router,
    extract::{Path, Query},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use langbank_web as site;

async fn language(Path(id): Path<String>) -> impl IntoResponse {
    match site::language(&id) {
        Some(html) => (StatusCode::OK, Html(html)),
        None => (StatusCode::NOT_FOUND, Html(site::not_found(&id))),
    }
}

/// GET carries the example links, which are short and worth sharing.
async fn identify(Query(query): Query<site::Query>) -> Html<String> {
    Html(site::identify_query(&query))
}

/// POST carries a pasted file. A 64 KiB header in a query string is a 414 from
/// the server before any of this runs, which is a poor answer to a reasonable
/// paste — so the form posts and only the examples use the URL.
async fn identify_posted(Form(query): Form<site::Query>) -> Html<String> {
    Html(site::identify_query(&query))
}

/// Fly's health check. Reports the registry sizes rather than a bare `ok`,
/// because a binary that started with an empty registry would otherwise pass.
async fn health() -> impl IntoResponse {
    let body = format!(
        r#"{{"ok":true,"languages":{},"ecosystems":{},"toolchains":{},"registries":{}}}"#,
        langbank::language_profiles().len(),
        langbank::ecosystem_profiles().len(),
        langbank::toolchains().len(),
        langbank::package_registries().len(),
    );
    ([(header::CONTENT_TYPE, "application/json")], body)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { Html(site::home()) }))
        .route("/languages", get(|| async { Html(site::languages()) }))
        .route("/languages/{id}", get(language))
        .route("/ecosystems", get(|| async { Html(site::ecosystems()) }))
        .route("/toolchains", get(|| async { Html(site::toolchains()) }))
        .route("/registries", get(|| async { Html(site::registries()) }))
        .route("/tools", get(|| async { Html(site::tools()) }))
        .route("/gaps", get(|| async { Html(site::gaps()) }))
        .route("/identify", get(identify).post(identify_posted))
        .route("/health", get(health))
        .fallback(|| async { (StatusCode::NOT_FOUND, Html(site::not_found("that page"))) });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("langbank-web: cannot bind port {port}: {error}");
            std::process::exit(1);
        }
    };
    println!(
        "langbank-web on http://0.0.0.0:{port} — {} languages, {} toolchains",
        langbank::language_profiles().len(),
        langbank::toolchains().len()
    );
    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("langbank-web: {error}");
        std::process::exit(1);
    }
}
