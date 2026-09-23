# Markdown Viewer v2

Windows / macOS / Linux で動く、表示専用の超軽量 Markdown ビューアーです（編集機能なし）。
[Tauri v2](https://v2.tauri.app/)（Rust + OS 標準 WebView）+ [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) + [highlight.js](https://highlightjs.org/) で構築しています。

参考にした前版 [markdown-viewer](../markdown-viewer)（WPF + WebView2 + Markdig）の機能をすべて継承したうえで、配布物と実行時メモリを大幅に軽くし、いくつかの改善を加えました。

## 前版との比較

| 項目 | 前版 (WPF) | v2 (Tauri) |
| --- | --- | --- |
| 配布物 | 単一 exe **62.4MB**（.NET ランタイム同梱） | 単一 exe **3.9MB**（ランタイム同梱なし） |
| メモリ使用量 (1 タブ) | 613MB（本体 208MB + WebView2 7 プロセス 406MB） | **373MB**（本体 27MB + WebView2 6 プロセス 346MB） |
| メモリ使用量 (5 タブ) | 868MB（本体 209MB + WebView2 11 プロセス 659MB） | **388MB**（本体 27MB + WebView2 6 プロセス 361MB） |
| WebView | タブごとに WebView2 を 1 つ生成 | ウィンドウ全体で **1 つ**（タブは DOM 内で管理） |
| 対応 OS | Windows 11 | Windows 10/11・macOS 10.15+・Linux (WebKitGTK 4.1) |
| インストール | 不要 | 不要（実行環境に必要なのは OS 標準の WebView のみ） |

メモリは Windows 11 上で同じ 5 ファイルを開き、アプリ本体と WebView2 子プロセスのワーキングセットを合計した実測値です（2026-09-22、リリースビルド）。WebView2 エンジン自体の常駐分（約 350MB）は両者共通で、v2 ではタブを増やしてもほとんど増えません。

## 機能

- **表示専用ビューアー** — 見出し / テーブル / コードブロック / 引用 / タスクリスト / 脚注 / 定義リスト / GFM アラート (`> [!NOTE]`) など GFM 相当の記法に対応
- **タブ表示** — 複数ファイルをタブで切り替えて表示。切り替えは瞬時でスクロール位置も保持されます（タブの中クリックで閉じる）
- **配色プリセット 9 種類** — ツールバー右上の「配色」から選択。タイトルバー（Windows 11）・ツールバー・本文・コードの色分けすべてに適用され、次回起動時にも引き継がれます
  | プリセット | 系統 |
  | --- | --- |
  | ライト / セピア / ソラライズド ライト | 明 |
  | ダーク / ソラライズド ダーク / ノルド / ドラキュラ / フォレスト / ハイコントラスト | 暗 |
- **ファイルの開き方** — 「開く...」ボタン (`Ctrl+O`、複数選択可) / **ウィンドウのどこにでも**ドラッグ＆ドロップ（複数可、ドロップ先を示すオーバーレイ表示付き）/ コマンドライン引数（「プログラムから開く」の関連付けに対応）
- **単一インスタンス** — 起動中に別のファイルを「プログラムから開く」しても、新しいウィンドウは開かず既存ウィンドウの新しいタブで開きます（macOS の Finder からのオープンにも対応）
- **自動再読み込み** — 開いているファイルが更新されると自動で再表示（スクロール位置は維持、`F5` / `Ctrl+R` で手動再読み込みも可能）
- **ページ内検索** — ツールバーの **「検索」ボタン** または `Ctrl+F` で検索バーを表示。入力に合わせて一致箇所をハイライトし、`Enter` / `Shift+Enter`（または `F3` / `Shift+F3`）で前後の一致へ移動、`Esc` で閉じます。大文字小文字は区別せず、検索語の空白は改行を含む任意の空白に一致します。自動再読み込み後やタブ切り替え後もハイライトは維持されます
- **構文ハイライト** — コードブロックの言語指定に応じて色分け（highlight.js の common 言語セット + PowerShell。配色プリセットごとにトークン色を用意）
- **ズーム** — `Ctrl+=` / `Ctrl+-` / `Ctrl+0`（`Ctrl+ホイール` でも可）。50〜300%、設定に保存されます
- **リンク** — 同一フォルダー配下の画像への相対リンクに対応。`.md` への相対リンクは新しいタブで開き、外部リンクは既定のブラウザーで開きます。ページ内アンカー (`#見出し`) にも対応
- **文字コード** — UTF-8 (BOM あり/なし)・UTF-16・Shift-JIS を自動判別（判別結果をステータスバーに表示）

macOS では `Ctrl` の代わりに `Cmd` を使います（`Ctrl+Tab` はそのまま）。

## キーボードショートカット

| キー | 動作 |
| --- | --- |
| `Ctrl+O` | ファイルを開く（複数選択可） |
| `F5` / `Ctrl+R` | アクティブなタブを再読み込み |
| `Ctrl+W` | アクティブなタブを閉じる（タブの中クリックでも可） |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | 次のタブ / 前のタブへ切り替え |
| `Ctrl+F` | ページ内検索バーを表示 |
| `Enter` / `Shift+Enter`、`F3` / `Shift+F3` | 次の一致 / 前の一致へ移動 |
| `Esc` | 検索バーを閉じる |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | 拡大 / 縮小 / 等倍 |

## 動作環境

- **Windows 10 (1803+) / 11** — WebView2 ランタイム（標準搭載）
- **macOS 10.15+** — 追加要件なし（WKWebView を使用）。CSS Custom Highlight API 非対応の macOS 14.1 以前では検索ハイライトを `<mark>` 方式に自動で切り替えます
- **Linux** — `webkit2gtk-4.1` がインストールされていること（AppImage も同様）

## 開発環境の準備（開発 PC のみ。配布先には不要）

| ツール | 用途 | 導入 |
| --- | --- | --- |
| Rust (stable) | バックエンドのビルド | `winget install --id Rustlang.Rustup`（他 OS は https://rustup.rs） |
| Node.js 18+ | Tauri CLI の実行のみ（フロントエンドにビルド工程はありません） | 導入済みなら `npm install` |
| Windows: Visual Studio Build Tools (C++) | Rust の MSVC リンカー | 導入済みなら不要 |
| macOS: Xcode Command Line Tools | 〃 | `xcode-select --install` |
| Linux | WebKitGTK など | `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` |

```powershell
npm install          # @tauri-apps/cli を取得
npm run dev          # 開発モードで起動 (ui/ の変更はリロードで反映)
```

### トラブルシューティング: `LNK1104: 'msvcrt.lib' を開くことができません`

複数の Visual Studio が入っていて、新しい方の C++ ツールセットが不完全（`link.exe` はあるが `VC\Tools\MSVC\<ver>\lib\x64` が無い）だと、Rust がそのリンカーを選んでリンクに失敗します。
完全なツールセットを `%USERPROFILE%\.cargo\config.toml` で明示してください（パスは環境に合わせて変更）。

```toml
# TOML のリテラル文字列 (シングルクォート) を使うとバックスラッシュをそのまま書けます
[target.x86_64-pc-windows-msvc]
linker = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\link.exe'
rustflags = ["-L", 'native=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\lib\x64']

[env]
# C/C++ を含む依存クレートのビルドスクリプト (cc-rs) 用
CC_x86_64-pc-windows-msvc = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\cl.exe'
CXX_x86_64-pc-windows-msvc = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\cl.exe'
AR_x86_64-pc-windows-msvc = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\lib.exe'
INCLUDE = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\include;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\winrt;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\cppwinrt'
```

根本対処は、不完全な方の Visual Studio に「MSVC C++ x64/x86 ビルド ツール」コンポーネントを追加する（または C++ ワークロードを外す）ことです。そうすればこの設定は不要になります。

## ビルドと配布

```powershell
# Windows: 素の exe を生成 (インストーラーは作らない)
npm run build        # = npx tauri build --no-bundle
# → src-tauri\target\release\MarkdownViewer.exe
```

```bash
# macOS: .app バンドル (Universal)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npx tauri build --target universal-apple-darwin --bundles app
# → src-tauri/target/universal-apple-darwin/release/bundle/macos/MarkdownViewer.app

# Linux: AppImage
npx tauri build --bundles appimage
```

`.github/workflows/build.yml` に 3 OS 分のビルドを行う GitHub Actions を用意しています（手動実行または `v*` タグの push で起動）。

- Windows: exe を任意の場所にコピーするだけで動作します。`.md` を右クリック →「プログラムから開く」→「別のプログラムを選択」でこの exe を指定すると、ダブルクリックで直接開けます
- macOS: 署名していない `.app` は初回に Gatekeeper の警告が出ます。Finder で右クリック →「開く」、または `xattr -dr com.apple.quarantine MarkdownViewer.app` で解除してください

## Rust 側の単体テスト

```powershell
cd src-tauri
cargo test
```

文字コード判別、見出し id の生成、`<script>` の無害化、画像配信のフォルダー境界チェック、相対リンク解決を検証します。

## 設定の保存先

`settings.json`（配色プリセット・ズーム率）

- 既定: Windows `%APPDATA%\jp.ne.njs.markdown-viewer\settings.json`、macOS `~/Library/Application Support/jp.ne.njs.markdown-viewer/settings.json`、Linux `~/.config/jp.ne.njs.markdown-viewer/settings.json`
- **ポータブル運用**: exe と同じフォルダーに `settings.json`（内容は `{}` でも可）を置いておくと、そのファイルに保存します

## セキュリティ上の設計

ウィンドウ全体が 1 つの WebView なので、Markdown 内の生 HTML はアプリと同じオリジンで描画されます。そのため次の多層防御を入れています。

- `tauri.conf.json` の CSP で `script-src 'self'`（インライン script・イベントハンドラー属性・`javascript:` を遮断）、`frame-src 'none'`、`object-src 'none'`
- Rust 側で `<script` を含む生 HTML チャンクはテキストとしてエスケープ（`render.rs`）
- 画像などのローカルファイル配信 (`mdfile` スキーム) は、開いているドキュメントのフォルダー配下に限定（`protocol.rs`）
- 外部 URL は `http` / `https` / `mailto` のみ既定アプリで開く（`commands.rs`）

## プロジェクト構成

| パス | 役割 |
| --- | --- |
| `ui/index.html` `app.css` `themes.css` | 画面 (ツールバー・タブ・検索バー・ステータスバー・ドロップ オーバーレイ)、レイアウト、配色プリセット 9 種 + 構文ハイライト色 |
| `ui/app.js` | タブ管理・ファイルを開く・自動再読み込み・D&D・ショートカット・配色・ズーム・リンク処理 |
| `ui/search.js` | ページ内検索 (CSS Custom Highlight API、非対応時は `<mark>` フォールバック) |
| `ui/vendor/` | highlight.js 11.11.2 (BSD-3-Clause) と PowerShell 言語定義、トークン色の割り当て CSS |
| `src-tauri/src/lib.rs` | Tauri ビルダー、プラグイン登録、起動引数、単一インスタンス、macOS のファイルオープン |
| `src-tauri/src/commands.rs` | IPC コマンド（開く / 再読み込み / 閉じる / リンク解決 / 外部 URL / 設定 / タイトルバー配色） |
| `src-tauri/src/render.rs` | Markdown → HTML (pulldown-cmark)、見出し id 付与、`<script>` 無害化 |
| `src-tauri/src/encoding.rs` | 文字コード判別 |
| `src-tauri/src/watcher.rs` | ファイル監視 (notify) と 300ms デバウンス |
| `src-tauri/src/protocol.rs` | `mdfile` カスタムスキーム（ローカル画像の配信） |
| `src-tauri/src/state.rs` | 開いているドキュメント・許可フォルダーの管理、パス正規化 |
| `src-tauri/src/settings.rs` | 設定の保存・読み込み（ポータブル対応） |
| `src-tauri/src/menu.rs` | macOS 用ネイティブメニュー |
| `src-tauri/tauri.conf.json` | ウィンドウ設定・CSP・バンドル設定 |
| `src-tauri/capabilities/default.json` | フロントエンドに許可する Tauri API |
| `sample.md` | 表示確認用サンプル |
| `design/icon.png` | アイコンの元画像（`npx tauri icon design/icon.png` で `src-tauri/icons/` を再生成） |
