//! ファイル監視。ドキュメントの親フォルダーを非再帰で監視し、対象ファイル名のイベントだけを
//! 300ms デバウンスして "doc-changed" イベントとしてフロントエンドへ通知する。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::{dir_key, path_key};

const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone, Serialize)]
struct DocChanged {
    path: String,
}

type FileMap = Arc<Mutex<HashMap<String, PathBuf>>>;

pub struct DocWatcher {
    watcher: RecommendedWatcher,
    /// 監視中フォルダー (キー → (実パス, 参照数))
    dirs: HashMap<String, (PathBuf, usize)>,
    /// 監視対象ファイル (キー → フロントエンドが知っている元のパス)
    files: FileMap,
}

impl DocWatcher {
    pub fn new(app: AppHandle) -> notify::Result<Self> {
        let files: FileMap = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<PathBuf>();

        let files_for_callback = Arc::clone(&files);
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            let map = files_for_callback.lock().unwrap();
            for path in &event.paths {
                if let Some(original) = map.get(&path_key(path)) {
                    let _ = tx.send(original.clone());
                }
            }
        })?;

        std::thread::Builder::new()
            .name("doc-watch-debounce".to_owned())
            .spawn(move || debounce_loop(rx, app))
            .expect("監視スレッドを起動できません");

        Ok(Self { watcher, dirs: HashMap::new(), files })
    }

    pub fn watch(&mut self, file: &Path) -> notify::Result<()> {
        let Some(dir) = file.parent() else { return Ok(()) };
        self.files.lock().unwrap().insert(path_key(file), file.to_path_buf());

        let entry = self
            .dirs
            .entry(dir_key(dir))
            .or_insert_with(|| (dir.to_path_buf(), 0));
        if entry.1 == 0 {
            self.watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }
        entry.1 += 1;
        Ok(())
    }

    pub fn unwatch(&mut self, file: &Path) {
        self.files.lock().unwrap().remove(&path_key(file));
        let Some(dir) = file.parent() else { return };
        let key = dir_key(dir);
        if let Some(entry) = self.dirs.get_mut(&key) {
            entry.1 = entry.1.saturating_sub(1);
            if entry.1 == 0 {
                let _ = self.watcher.unwatch(&entry.0);
                self.dirs.remove(&key);
            }
        }
    }
}

fn debounce_loop(rx: Receiver<PathBuf>, app: AppHandle) {
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
    loop {
        let timeout = if pending.is_empty() {
            Duration::from_secs(3600)
        } else {
            Duration::from_millis(50)
        };
        match rx.recv_timeout(timeout) {
            Ok(path) => {
                pending.insert(path, Instant::now());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let now = Instant::now();
        let ready: Vec<PathBuf> = pending
            .iter()
            .filter(|(_, at)| now.duration_since(**at) >= DEBOUNCE)
            .map(|(path, _)| path.clone())
            .collect();
        for path in ready {
            pending.remove(&path);
            let _ = app.emit(
                "doc-changed",
                DocChanged { path: path.to_string_lossy().into_owned() },
            );
        }
    }
}
