//! フロントエンドから invoke される IPC コマンド。

use std::path::{Path, PathBuf};
use std::time::Duration;

use percent_encoding::percent_decode_str;
use serde::Serialize;
use tauri::{AppHandle, State, Theme, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use crate::settings::{self, Settings};
use crate::state::{normalize_path, SharedState};
use crate::{encoding, render};

pub const MARKDOWN_EXTENSIONS: [&str; 6] = ["md", "markdown", "mdown", "mkd", "mdtxt", "txt"];

#[derive(Serialize)]
pub struct Document {
    pub path: String,
    pub name: String,
    pub dir: String,
    pub html: String,
    pub encoding: &'static str,
}

fn to_abs(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("パスが空です".to_owned());
    }
    normalize_path(Path::new(path)).map_err(|e| format!("パスを解決できません: {e}"))
}

/// エディターの保存直後などでロック中の場合に備え、100ms × 最大 4 回リトライする。
fn read_with_retry(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut attempt = 0;
    loop {
        match std::fs::read(path) {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt < 4 && e.kind() != std::io::ErrorKind::NotFound => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
}

fn document_from(path: &Path, html: String, encoding: &'static str) -> Document {
    Document {
        path: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        dir: path
            .parent()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        html,
        encoding,
    }
}

fn build_document(path: &Path, bytes: &[u8]) -> Document {
    let decoded = encoding::decode(bytes);
    document_from(path, render::render(&decoded.text), decoded.encoding)
}

pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| MARKDOWN_EXTENSIONS.iter().any(|m| m.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn open_document(path: String, state: State<'_, SharedState>) -> Result<Document, String> {
    let abs = to_abs(&path)?;
    if !abs.is_file() {
        return Err(format!("ファイルが見つかりません: {}", abs.display()));
    }
    let bytes = read_with_retry(&abs).map_err(|e| format!("読み込めませんでした: {e}"))?;
    let doc = build_document(&abs, &bytes);
    state.lock().unwrap().register(&abs);
    Ok(doc)
}

/// 再読み込み。読めなくなった場合はエラー内容を本文として返す (タブは維持する)。
#[tauri::command]
pub async fn reload_document(path: String) -> Result<Document, String> {
    let abs = to_abs(&path)?;
    match read_with_retry(&abs) {
        Ok(bytes) => Ok(build_document(&abs, &bytes)),
        Err(e) => {
            let markdown = format!(
                "# 読み込みエラー\n\n`{}` を読み込めませんでした。\n\n```\n{}\n```",
                abs.display(),
                e
            );
            Ok(document_from(&abs, render::render(&markdown), ""))
        }
    }
}

#[tauri::command]
pub fn close_document(path: String, state: State<'_, SharedState>) -> Result<(), String> {
    let abs = to_abs(&path)?;
    state.lock().unwrap().unregister(&abs);
    Ok(())
}

#[tauri::command]
pub fn render_text(markdown: String) -> String {
    render::render(&markdown)
}

/// 相対リンクをドキュメントのフォルダー基準で解決し、存在する Markdown ファイルなら絶対パスを返す。
#[tauri::command]
pub fn resolve_link(dir: String, href: String) -> Option<String> {
    let cleaned = href.split(['#', '?']).next().unwrap_or("").trim();
    let decoded = percent_decode_str(cleaned).decode_utf8_lossy();
    if decoded.is_empty() {
        return None;
    }
    let target = normalize_path(&Path::new(&dir).join(&*decoded)).ok()?;
    if target.is_file() && is_markdown_file(&target) {
        Some(target.to_string_lossy().into_owned())
    } else {
        None
    }
}

#[tauri::command]
pub fn open_external(url: String, app: AppHandle) -> Result<(), String> {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    let allowed =
        lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:");
    if !allowed {
        return Err("許可されていない URL スキームです".to_owned());
    }
    app.opener()
        .open_url(trimmed, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Settings {
    settings::load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    settings::save(&app, &settings)
}

/// ウィンドウのライト/ダークと、Windows ではタイトルバーの配色をテーマに合わせる。
#[tauri::command]
pub fn set_chrome(window: WebviewWindow, is_dark: bool, bg: String, fg: String) -> Result<(), String> {
    window
        .set_theme(Some(if is_dark { Theme::Dark } else { Theme::Light }))
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    apply_dwm(&window, &bg, &fg);
    #[cfg(not(windows))]
    let _ = (bg, fg);
    Ok(())
}

/// 起動引数 / 関連付けで渡されたファイルを 1 回だけ返す。以後はイベント配信に切り替える。
#[tauri::command]
pub fn take_startup_files(state: State<'_, SharedState>) -> Vec<String> {
    let mut st = state.lock().unwrap();
    st.frontend_ready = true;
    std::mem::take(&mut st.startup_files)
}

#[cfg(windows)]
fn apply_dwm(window: &WebviewWindow, bg: &str, fg: &str) {
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const core::ffi::c_void,
            size: u32,
        ) -> i32;
    }
    const DWMWA_CAPTION_COLOR: u32 = 35;
    const DWMWA_TEXT_COLOR: u32 = 36;

    let Ok(hwnd) = window.hwnd() else { return };
    let hwnd = hwnd.0 as isize;
    for (attribute, hex) in [(DWMWA_CAPTION_COLOR, bg), (DWMWA_TEXT_COLOR, fg)] {
        if let Some(color) = colorref(hex) {
            // Windows 10 では属性が未対応でエラーになるが、無視して構わない
            unsafe {
                DwmSetWindowAttribute(hwnd, attribute, &color as *const u32 as *const _, 4);
            }
        }
    }
}

/// "#RRGGBB" → COLORREF (0x00BBGGRR)
#[cfg(windows)]
fn colorref(hex: &str) -> Option<u32> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    let (r, g, b) = ((value >> 16) & 0xFF, (value >> 8) & 0xFF, value & 0xFF);
    Some((b << 16) | (g << 8) | r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_extension_check() {
        assert!(is_markdown_file(Path::new("a.md")));
        assert!(is_markdown_file(Path::new("a.MD")));
        assert!(is_markdown_file(Path::new("a.txt")));
        assert!(!is_markdown_file(Path::new("a.png")));
        assert!(!is_markdown_file(Path::new("noext")));
    }

    #[test]
    fn resolve_link_rejects_missing_and_non_markdown() {
        let dir = std::env::temp_dir().join("mdv2-link-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("other.md"), "# x").unwrap();
        std::fs::write(dir.join("pic.png"), b"\x89PNG").unwrap();
        let dir_s = dir.to_string_lossy().into_owned();

        assert!(resolve_link(dir_s.clone(), "other.md#sec".into()).is_some());
        assert!(resolve_link(dir_s.clone(), "./other.md?x=1".into()).is_some());
        assert!(resolve_link(dir_s.clone(), "pic.png".into()).is_none());
        assert!(resolve_link(dir_s.clone(), "missing.md".into()).is_none());
        assert!(resolve_link(dir_s, "".into()).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn colorref_swaps_channels() {
        assert_eq!(colorref("#0067C0"), Some(0x00C0_6700));
        assert_eq!(colorref("bad"), None);
    }
}
