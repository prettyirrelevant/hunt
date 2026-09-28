use std::sync::Arc;

use axum::{
    Extension, Router,
    extract::{Multipart, Path, Request},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use rust_embed::RustEmbed;

use crate::{
    app::App,
    ui::shell::{App as Ui, shell},
};

#[derive(RustEmbed)]
#[folder = "target/site"]
#[allow_missing = true]
#[exclude = "*.gz"]
#[exclude = "*.br"]
struct Site;

pub fn router(app: Arc<App>) -> Router {
    let options = LeptosOptions::builder()
        .output_name("hunt")
        .site_pkg_dir("pkg")
        .site_addr(std::net::SocketAddr::from(([127, 0, 0, 1], app.config.port)))
        .build();
    let context = {
        let app = Arc::clone(&app);
        move || provide_context(Arc::clone(&app))
    };
    let routes = generate_route_list(Ui);
    Router::new()
        .route("/documents/{id}/{file}", get(document))
        .route("/documents/{id}/recording/{file}", get(recording))
        .route("/you/cv", post(upload_cv))
        .route("/pkg/{*path}", get(asset))
        .route("/vendor/{*path}", get(asset))
        .route("/fonts/{*path}", get(asset))
        .leptos_routes_with_context(&options, routes, context, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .layer(middleware::from_fn(same_origin))
        .layer(Extension(app))
        .with_state(options)
}

/// Rejects a foreign `Host` (DNS rebinding) or `Origin` (cross-site posts).
async fn same_origin(request: Request, next: Next) -> Response {
    let local = |value: &str| {
        let host = value.trim_start_matches("http://");
        host.starts_with("localhost:") || host.starts_with("127.0.0.1:") || host == "localhost" || host == "127.0.0.1"
    };
    let allowed = {
        let header = |name| request.headers().get(name).and_then(|v| v.to_str().ok());
        header(header::HOST).is_some_and(local) && header(header::ORIGIN).is_none_or(local)
    };
    if allowed { next.run(request).await } else { StatusCode::FORBIDDEN.into_response() }
}

async fn asset(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    match Site::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn document(Extension(app): Extension<Arc<App>>, Path((id, file)): Path<(i64, String)>) -> Response {
    if !matches!(file.as_str(), "cv.pdf" | "cover-letter.pdf") {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(app.config.documents().join(id.to_string()).join(&file)).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/pdf")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn recording(Extension(app): Extension<Arc<App>>, Path((id, file)): Path<(i64, String)>) -> Response {
    let safe =
        file.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) && !file.starts_with('.');
    let mime = match file.rsplit('.').next() {
        Some("webm") => "video/webm",
        Some("png") => "image/png",
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if !safe {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(app.config.documents().join(id.to_string()).join("recording").join(&file)).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, mime)], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn upload_cv(Extension(app): Extension<Arc<App>>, mut form: Multipart) -> Response {
    while let Ok(Some(field)) = form.next_field().await {
        if field.name() != Some("cv") {
            continue;
        }
        let name = field.file_name().unwrap_or("cv.pdf").to_string();
        let result = match field.bytes().await {
            Ok(bytes) => crate::profile::service::import_cv(&app, &name, &bytes).await,
            Err(err) => Err(err.into()),
        };
        if let Err(err) = result {
            tracing::error!("CV import failed: {err:#}");
            return (StatusCode::BAD_REQUEST, format!("Could not read that CV: {err:#}")).into_response();
        }
    }
    Redirect::to("/you").into_response()
}
