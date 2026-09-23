# サンプルドキュメント

これは **Markdown Viewer v2** の表示確認用サンプルです。目次: [テキスト装飾](#テキスト装飾) / [コードブロック](#コードブロック) / [テーブル](#テーブル) / [アラート](#アラート)

## テキスト装飾

*イタリック*、**太字**、~~取り消し線~~、`インラインコード`、[外部リンク](https://example.com)、脚注[^1]、上付き ^注^ と下付き ~添字~ 。

## コードブロック

```csharp
public static void Main(string[] args)
{
    // 構文ハイライトの確認
    Console.WriteLine("こんにちは、Markdown Viewer!");
}
```

```javascript
const tabs = new Map();
export async function open(path) {
  const doc = await invoke('open_document', { path });
  tabs.set(path, doc);
  return doc.html.length > 0;
}
```

```powershell
Get-ChildItem -Path . -Filter *.md | ForEach-Object { Write-Host $_.Name }
```

```json
{ "theme": "nord", "zoom": 120 }
```

```diff
- 旧: WPF + WebView2 (タブごとに WebView2)、単一 exe 約 62MB
+ 新: Tauri v2 + OS 標準 WebView (WebView は 1 つ)、単一 exe 数 MB
```

## テーブル

| プリセット | 系統 | 特徴 |
| --- | --- | --- |
| ライト | 明 | 標準的な白背景 |
| ダーク | 暗 | GitHub Dark 風 |
| セピア | 明 | 目に優しい暖色 |
| ノルド | 暗 | 北欧カラー |

## 引用とリスト

> 引用ブロックのサンプルです。
> 複数行にわたる引用も表示できます。

1. 番号付きリスト
2. 二番目の項目
   - 入れ子の箇条書き
   - もう一つ

## タスクリスト

- [x] ビューアー機能
- [x] 配色プリセット 9 種類
- [x] 検索ボタン・ズーム・構文ハイライト・単一インスタンス化
- [ ] 編集機能（対象外）

## アラート

> [!NOTE]
> GFM のアラート記法にも対応しています。

> [!WARNING]
> ファイルが削除されると、タブには読み込みエラーが表示されます。

## 画像と相対リンク

同じフォルダー配下の画像は相対パスで表示できます:

![アプリアイコン](design/icon.png)

相対リンク先が Markdown なら新しいタブで開きます: [README](README.md)

## 定義リスト

Tauri
: OS 標準の WebView を使う軽量なデスクトップアプリ フレームワーク。

pulldown-cmark
: Rust 製の CommonMark / GFM パーサー。

---

以上です。

[^1]: これは脚注です。クリックすると本文と脚注の間を移動できます。
