//! ユーザー設定 (配色プリセット・ズーム率)。
//! exe と同じフォルダーに settings.json があればポータブル運用としてそれを使い、無ければ OS の設定フォルダーに保存する。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const ZOOM_MIN: u32 = 50;
pub const ZOOM_MAX: u32 = 300;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub zoom: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "light".to_owned(), zoom: 100 }
    }
}

pub fn settings_path(app: &AppHandle) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("settings.json");
            if portable.is_file() {
                return portable;
            }
        }
    }
    match app.path().app_config_dir() {
        Ok(dir) => dir.join("settings.json"),
        Err(_) => PathBuf::from("settings.json"),
    }
}

pub fn load(app: &AppHandle) -> Settings {
    let mut settings = std::fs::read_to_string(settings_path(app))
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default();
    settings.zoom = settings.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    settings
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
