//! `mdfile` カスタムスキーム: 開いているドキュメントのフォルダー配下にあるローカルファイル (主に画像) を配信する。
//! Windows では `http://mdfile.localhost/<encoded path>`、macOS/Linux では `mdfile://localhost/<encoded path>` として届く。

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use tauri::http::{header, Request, Response, StatusCode};

use crate::state::{normalize_path, AppState};

type Body = Cow<'static, [u8]>;

pub fn handle(state: &AppState, request: &Request<Vec<u8>>) -> Response<Body> {
    let raw = request.uri().path().trim_start_matches('/');
    let decoded = percent_decode_str(raw).decode_utf8_lossy();
    if decoded.is_empty() {
        return status(StatusCode::BAD_REQUEST);
    }

    let path = match normalize_path(&PathBuf::from(&*decoded)) {
        Ok(path) => path,
        Err(_) => return status(StatusCode::BAD_REQUEST),
    };
    if !state.is_allowed(&path) {
        return status(StatusCode::FORBIDDEN);
    }

    match std::fs::read(&path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_for(&path))
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Cow::Owned(bytes))
            .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
        Err(_) => status(StatusCode::NOT_FOUND),
    }
}

fn status(code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(code)
        .body(Cow::Borrowed(&[][..]))
        .expect("空レスポンスの構築に失敗")
}

pub fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "tif" | "tiff" => "image/tiff",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_detection() {
        assert_eq!(mime_for(Path::new("a.PNG")), "image/png");
        assert_eq!(mime_for(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(mime_for(Path::new("a.bin")), "application/octet-stream");
    }
}
