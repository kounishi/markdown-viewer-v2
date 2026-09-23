//! macOS 用ネイティブメニュー。Cmd 系ショートカットはここで受けてフロントエンドへ "menu-action" として通知する。
//! (既定メニューの「Close Window」がアプリのウィンドウを閉じてしまうため、必ず自前で定義する)

use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{App, Emitter};

#[derive(Clone, Serialize)]
struct MenuAction {
    id: String,
}

pub fn install(app: &App) -> tauri::Result<()> {
    let handle = app.handle();
    let item = |id: &str, text: &str, accelerator: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accelerator)
            .build(handle)
    };

    let app_menu = SubmenuBuilder::new(handle, "Markdown Viewer")
        .about(None)
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(handle, "ファイル")
        .item(&item("open", "開く...", "Cmd+O")?)
        .separator()
        .item(&item("close-tab", "タブを閉じる", "Cmd+W")?)
        .build()?;

    let edit_menu = SubmenuBuilder::new(handle, "編集")
        .copy()
        .select_all()
        .separator()
        .item(&item("find", "検索", "Cmd+F")?)
        .item(&item("find-next", "次を検索", "Cmd+G")?)
        .item(&item("find-prev", "前を検索", "Shift+Cmd+G")?)
        .build()?;

    let view_menu = SubmenuBuilder::new(handle, "表示")
        .item(&item("reload", "再読み込み", "Cmd+R")?)
        .separator()
        .item(&item("zoom-in", "拡大", "Cmd+Equal")?)
        .item(&item("zoom-out", "縮小", "Cmd+Minus")?)
        .item(&item("zoom-reset", "等倍", "Cmd+Digit0")?)
        .separator()
        .item(&item("next-tab", "次のタブ", "Ctrl+Tab")?)
        .item(&item("prev-tab", "前のタブ", "Ctrl+Shift+Tab")?)
        .build()?;

    let window_menu = SubmenuBuilder::new(handle, "ウィンドウ")
        .minimize()
        .maximize()
        .build()?;

    let menu = MenuBuilder::new(handle)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu])
        .build()?;
    app.set_menu(menu)?;

    app.on_menu_event(|app, event| {
        let _ = app.emit("menu-action", MenuAction { id: event.id().0.clone() });
    });
    Ok(())
}
