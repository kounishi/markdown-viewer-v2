//! Markdown Viewer v2 — Tauri v2 バックエンド。

mod commands;
mod encoding;
#[cfg(target_os = "macos")]
mod menu;
mod protocol;
mod render;
mod settings;
mod state;
mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{Emitter, Manager};

use state::{normalize_path, AppState, OpenFiles, SharedState};

/// 引数のうち存在するファイルだけを正規化した絶対パスで返す。相対パスは `base` から解決する。
fn collect_files<I: IntoIterator<Item = String>>(args: I, base: Option<&Path>) -> Vec<String> {
    args.into_iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|arg| {
            let path = PathBuf::from(&arg);
            let path = match (path.is_absolute(), base) {
                (false, Some(base)) => base.join(path),
                _ => path,
            };
            normalize_path(&path).ok()
        })
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// フロントエンドが準備できていれば "open-files" イベントで渡し、まだなら起動ファイルとして保留する。
fn deliver_files(app: &tauri::AppHandle, files: Vec<String>) {
    if files.is_empty() {
        return;
    }
    let state = app.state::<SharedState>();
    let mut st = state.lock().unwrap();
    if st.frontend_ready {
        let _ = app.emit("open-files", OpenFiles { paths: files });
    } else {
        st.startup_files.extend(files);
    }
}

pub fn run() {
    let startup_files = collect_files(std::env::args().skip(1), None);

    tauri::Builder::default()
        // 単一インスタンス化は他のプラグインより先に登録する必要がある
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let files = collect_files(args.into_iter().skip(1), Some(Path::new(&cwd)));
            deliver_files(app, files);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(AppState::new(startup_files)))
        .register_uri_scheme_protocol("mdfile", |ctx, request| {
            let state = ctx.app_handle().state::<SharedState>();
            let st = state.lock().unwrap();
            protocol::handle(&st, &request)
        })
        .setup(|app| {
            let watcher = watcher::DocWatcher::new(app.handle().clone())?;
            app.state::<SharedState>().lock().unwrap().watcher = Some(watcher);
            #[cfg(target_os = "macos")]
            menu::install(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_document,
            commands::reload_document,
            commands::close_document,
            commands::render_text,
            commands::resolve_link,
            commands::open_external,
            commands::load_settings,
            commands::save_settings,
            commands::set_chrome,
            commands::take_startup_files,
        ])
        .build(tauri::generate_context!())
        .expect("Tauri アプリの構築に失敗しました")
        .run(|_app, _event| {
            // macOS: Finder からの「このアプリで開く」/ ダブルクリック起動
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &_event {
                let files: Vec<String> = urls
                    .iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .filter(|path| path.is_file())
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                deliver_files(_app, files);
            }
        });
}
