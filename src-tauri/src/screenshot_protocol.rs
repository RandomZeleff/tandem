//! `tandem-shot://localhost/<instance>%2F<file>[?thumb]`: serves an instance's
//! screenshots (or their thumbnails) to `<img>` tags, so the webview loads only
//! what is on screen and never sees an arbitrary file path.

use std::path::{Component, Path};

use tandem_core::screenshots;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::AppState;

pub const SCHEME: &str = "tandem-shot";

pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let data = ctx.app_handle().state::<AppState>().ctx.data.clone();
    let uri = request.uri().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = percent_encoding::percent_decode_str(uri.path().trim_start_matches('/'))
            .decode_utf8_lossy()
            .into_owned();
        let thumb = uri
            .query()
            .is_some_and(|q| q.split('&').any(|p| p == "thumb"));
        let file = path.split_once('/').and_then(|(instance, file_name)| {
            if !is_plain(instance) {
                return None;
            }
            let game_dir = data.instance_dir(instance);
            let result = if thumb {
                screenshots::thumbnail(&data, instance, &game_dir, file_name)
            } else {
                screenshots::path(&game_dir, file_name)
            };
            result
                .inspect_err(|err| tracing::debug!(%path, error = %err, "screenshot not served"))
                .ok()
        });
        let response = match file.map(|f| (std::fs::read(&f), f)) {
            Some((Ok(bytes), f)) => Response::builder()
                .header(header::CONTENT_TYPE, content_type(&f))
                // URLs carry the screenshot's date (`v=`), so a cached copy never goes stale.
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                .body(bytes),
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new()),
        };
        if let Ok(response) = response {
            responder.respond(response);
        }
    });
}

/// A single plain path component, like an instance id.
fn is_plain(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    )
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        _ => "image/jpeg",
    }
}
