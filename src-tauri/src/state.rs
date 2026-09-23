//! アプリ全体の共有状態 (開いているドキュメント、画像配信を許可するフォルダー、起動時に渡されたファイル)。

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::watcher::DocWatcher;

#[derive(Clone, Serialize)]
pub struct OpenFiles {
    pub paths: Vec<String>,
}

pub struct AppState {
    /// 起動引数 / 関連付け起動で渡され、まだフロントエンドへ渡していないファイル。
    pub startup_files: Vec<String>,
    /// フロントエンドがイベントを受け取れる状態になったか (take_startup_files 呼び出し後)。
    pub frontend_ready: bool,
    /// 開いているドキュメントのキー ([`path_key`])。
    pub docs: HashSet<String>,
    /// 画像配信を許可するフォルダー (キー → 参照数)。
    pub allowed_dirs: HashMap<String, usize>,
    pub watcher: Option<DocWatcher>,
}

pub type SharedState = Mutex<AppState>;

impl AppState {
    pub fn new(startup_files: Vec<String>) -> Self {
        Self {
            startup_files,
            frontend_ready: false,
            docs: HashSet::new(),
            allowed_dirs: HashMap::new(),
            watcher: None,
        }
    }

    /// ドキュメントを登録する。既に登録済みなら何もしない (冪等)。
    pub fn register(&mut self, path: &Path) {
        if !self.docs.insert(path_key(path)) {
            return;
        }
        if let Some(dir) = path.parent() {
            *self.allowed_dirs.entry(dir_key(dir)).or_insert(0) += 1;
        }
        if let Some(watcher) = self.watcher.as_mut() {
            let _ = watcher.watch(path);
        }
    }

    pub fn unregister(&mut self, path: &Path) {
        if !self.docs.remove(&path_key(path)) {
            return;
        }
        if let Some(dir) = path.parent() {
            let key = dir_key(dir);
            if let Some(count) = self.allowed_dirs.get_mut(&key) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.allowed_dirs.remove(&key);
                }
            }
        }
        if let Some(watcher) = self.watcher.as_mut() {
            watcher.unwatch(path);
        }
    }

    /// 開いているドキュメントのフォルダー配下 (サブフォルダー含む) のファイルだけ配信を許可する。
    pub fn is_allowed(&self, path: &Path) -> bool {
        let key = path_key(path);
        self.allowed_dirs.keys().any(|dir| {
            key.len() > dir.len()
                && key.starts_with(dir.as_str())
                && key[dir.len()..].starts_with(std::path::MAIN_SEPARATOR)
        })
    }
}

/// 絶対パス化し、`.` / `..` を字句的に解決する (シンボリックリンクは解決しない)。
pub fn normalize_path(path: &Path) -> std::io::Result<PathBuf> {
    let abs = std::path::absolute(path)?;
    let mut out = PathBuf::new();
    for component in abs.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    Ok(out)
}

/// パス比較用キー。Windows では大文字小文字を無視する。
pub fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.into_owned()
    }
}

/// フォルダー用キー。末尾の区切り文字を除く (ルート `C:\` も `c:` になる)。
pub fn dir_key(dir: &Path) -> String {
    let key = path_key(dir);
    key.trim_end_matches(std::path::MAIN_SEPARATOR).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_doc() -> PathBuf {
        let dir = std::env::temp_dir().join("mdv2-state-test");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        dir.join("doc.md")
    }

    #[test]
    fn allows_only_files_under_open_document_dirs() {
        let doc = normalize_path(&temp_doc()).unwrap();
        let dir = doc.parent().unwrap().to_path_buf();
        let mut state = AppState::new(Vec::new());
        assert!(!state.is_allowed(&dir.join("img.png")));

        state.register(&doc);
        assert!(state.is_allowed(&dir.join("img.png")));
        assert!(state.is_allowed(&dir.join("sub").join("deep.png")));
        assert!(!state.is_allowed(&dir.parent().unwrap().join("outside.png")));
        // 同名で始まる別フォルダー (mdv2-state-test-evil) は許可しない
        let sibling = dir.parent().unwrap().join(format!(
            "{}-evil",
            dir.file_name().unwrap().to_string_lossy()
        ));
        assert!(!state.is_allowed(&sibling.join("x.png")));

        state.unregister(&doc);
        assert!(!state.is_allowed(&dir.join("img.png")));
    }

    #[test]
    fn register_is_idempotent() {
        let doc = normalize_path(&temp_doc()).unwrap();
        let mut state = AppState::new(Vec::new());
        state.register(&doc);
        state.register(&doc);
        assert_eq!(state.allowed_dirs.values().copied().sum::<usize>(), 1);
        state.unregister(&doc);
        assert!(state.allowed_dirs.is_empty());
    }

    #[test]
    fn normalize_resolves_parent_components() {
        let doc = normalize_path(&temp_doc()).unwrap();
        let dir = doc.parent().unwrap();
        let sneaky = dir.join("sub").join("..").join("..").join("secret.txt");
        let normalized = normalize_path(&sneaky).unwrap();
        assert_eq!(normalized, dir.parent().unwrap().join("secret.txt"));
    }
}
