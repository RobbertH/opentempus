//! Serve the embedded single-page web app. In a release build the `web/dist`
//! folder is compiled into the binary; during development you run Vite and
//! proxy `/api` to this server instead.

use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../web/dist/"]
struct Assets;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") || path.starts_with("feeds/") {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if let Some(file) = Assets::get(path) {
        return file_response(path, file.data.into_owned(), true);
    }
    match Assets::get("index.html") {
        Some(index) => file_response("index.html", index.data.into_owned(), false),
        None => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            "<h1>OpenTempus API is running</h1><p>The web app was not bundled into this binary. Build it with <code>cd web && pnpm build</code> and rebuild the server, or run the Vite dev server.</p>",
        )
            .into_response(),
    }
}

fn file_response(path: &str, data: Vec<u8>, immutable: bool) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if immutable && path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    (StatusCode::OK, [(header::CONTENT_TYPE, mime.as_ref().to_string()), (header::CACHE_CONTROL, cache.to_string())], data)
        .into_response()
}
