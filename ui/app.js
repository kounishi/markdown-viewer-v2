/* Markdown Viewer v2 — フロントエンド本体。
 * タブ (1 タブ = 1 つの .pane 要素)、検索バー、ウィンドウ全面のドラッグ＆ドロップ、ショートカット、配色、ズームを担当する。
 * Markdown → HTML 変換やファイル監視は Rust 側 (src-tauri) のコマンド/イベントを使う。 */
(() => {
  'use strict';

  const tauri = window.__TAURI__;
  if (!tauri) {
    document.body.textContent = 'Tauri API を初期化できませんでした。';
    return;
  }
  const { invoke, convertFileSrc } = tauri.core;
  const { listen } = tauri.event;
  const appWindow = tauri.window.getCurrentWindow();
  const webview = tauri.webview.getCurrentWebview();
  const dialog = tauri.dialog;

  const IS_WIN = navigator.userAgent.includes('Windows');
  const IS_MAC = navigator.userAgent.includes('Macintosh') || /^Mac/.test(navigator.platform);
  const SEP = IS_WIN ? '\\' : '/';
  const MOD = IS_MAC ? 'Cmd' : 'Ctrl';
  const ZOOM = { min: 50, max: 300, step: 10 };
  const HINT = 'ファイルを開くか、ここへドラッグ＆ドロップしてください';
  const SCHEME_RE = /^[a-z][a-z0-9+.-]*:/i;

  const THEMES = [
    { id: 'light', name: 'ライト', dark: false },
    { id: 'dark', name: 'ダーク', dark: true },
    { id: 'sepia', name: 'セピア', dark: false },
    { id: 'solarized-light', name: 'ソラライズド ライト', dark: false },
    { id: 'solarized-dark', name: 'ソラライズド ダーク', dark: true },
    { id: 'nord', name: 'ノルド', dark: true },
    { id: 'dracula', name: 'ドラキュラ', dark: true },
    { id: 'forest', name: 'フォレスト', dark: true },
    { id: 'high-contrast', name: 'ハイコントラスト', dark: true },
  ];

  const WELCOME_MD = [
    '# Markdown Viewer へようこそ 👋',
    '',
    'Markdown ファイルを表示するには、次のいずれかの方法を使ってください。',
    '',
    '- ツールバーの **「開く...」** ボタン（`Ctrl+O`、複数選択可）',
    '- このウィンドウの**どこにでも**ファイルを**ドラッグ＆ドロップ**（複数可）',
    '- `.md` ファイルを右クリック →「プログラムから開く」でこのアプリを指定（起動中なら同じウィンドウの新しいタブで開きます）',
    '',
    '## 機能',
    '',
    '| 機能 | 説明 |',
    '| --- | --- |',
    '| タブ表示 | 複数のファイルをタブで切り替えて表示できます |',
    '| 配色プリセット | 右上の「配色」から **9 種類** を選択できます |',
    '| 自動再読み込み | 表示中のファイルが更新されると自動で反映します |',
    '| ページ内検索 | ツールバーの **「検索」** ボタンまたは `Ctrl+F` で検索バーを表示。一致箇所をハイライトし、前後の一致へ移動できます |',
    '| 構文ハイライト | コードブロックの言語指定（csharp、json、powershell など）に応じて色分けします |',
    '| ズーム | `Ctrl+=` / `Ctrl+-` で本文を拡大・縮小、`Ctrl+0` で等倍に戻します（`Ctrl+ホイール` でも可） |',
    '| 相対リンク | 同じフォルダー内の画像に対応。`.md` へのリンクは新しいタブで開きます |',
    '| 外部リンク | 既定のブラウザーで開きます |',
    '',
    '## キーボードショートカット',
    '',
    '| キー | 動作 |',
    '| --- | --- |',
    '| `Ctrl+O` | ファイルを開く |',
    '| `F5` / `Ctrl+R` | 再読み込み |',
    '| `Ctrl+W` | アクティブなタブを閉じる（タブの中クリックでも可） |',
    '| `Ctrl+Tab` / `Ctrl+Shift+Tab` | タブの切り替え |',
    '| `Ctrl+F` | ページ内検索 |',
    '| `Enter` / `Shift+Enter`、`F3` / `Shift+F3` | 次の一致 / 前の一致へ移動 |',
    '| `Esc` | 検索を閉じる |',
    '| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | 拡大 / 縮小 / 等倍 |',
    '',
    '> 選んだ配色とズーム率は次回起動時にも引き継がれます。',
  ]
    .join('\n')
    .replace(/Ctrl\+(?!Tab|Shift\+Tab)/g, `${MOD}+`);

  const $ = (id) => document.getElementById(id);
  const el = {
    btnOpen: $('btn-open'),
    btnReload: $('btn-reload'),
    btnSearch: $('btn-search'),
    themeSelect: $('theme-select'),
    tabstrip: $('tabstrip'),
    searchbar: $('searchbar'),
    searchInput: $('search-input'),
    searchCount: $('search-count'),
    searchPrev: $('search-prev'),
    searchNext: $('search-next'),
    searchClose: $('search-close'),
    content: $('content'),
    welcome: $('welcome'),
    statusText: $('status-text'),
    statusEncoding: $('status-encoding'),
    statusZoom: $('status-zoom'),
    dropOverlay: $('drop-overlay'),
  };

  /** @type {{ tabs: Tab[], active: Tab|null, settings: {theme:string, zoom:number}, imageVersion: number, saveTimer: number }} */
  const state = {
    tabs: [],
    active: null,
    settings: { theme: 'light', zoom: 100 },
    imageVersion: 0,
    saveTimer: 0,
  };

  // ---- パス補助 ----------------------------------------------------------------

  const pathKey = (p) => (IS_WIN ? p.replace(/\//g, '\\').toLowerCase() : p);
  const findTab = (p) => {
    const key = pathKey(p);
    return state.tabs.find((t) => pathKey(t.path) === key) || null;
  };
  const activePane = () => (state.active ? state.active.pane : el.welcome);
  const isRelativeUrl = (u) => !!u && !u.startsWith('#') && !u.startsWith('//') && !SCHEME_RE.test(u);

  function joinPath(dir, rel) {
    let r = rel.replace(/^[\\/]+/, '');
    if (IS_WIN) r = r.replace(/\//g, '\\');
    return dir.replace(/[\\/]+$/, '') + SEP + r;
  }

  // ---- タブ管理 ----------------------------------------------------------------

  /**
   * @typedef {{ path: string, name: string, dir: string, encoding: string,
   *             pane: HTMLElement, tabEl: HTMLElement, scrollTop: number }} Tab
   */

  function createTab(doc) {
    const pane = document.createElement('div');
    pane.className = 'pane';
    pane.tabIndex = -1;
    pane.hidden = true;
    const article = document.createElement('article');
    article.className = 'markdown-body';
    pane.appendChild(article);
    el.content.appendChild(pane);

    const tabEl = document.createElement('div');
    tabEl.className = 'tab';
    tabEl.title = doc.path;
    tabEl.setAttribute('role', 'tab');
    const name = document.createElement('span');
    name.className = 'tab-name';
    name.textContent = doc.name;
    const close = document.createElement('button');
    close.className = 'close';
    close.type = 'button';
    close.title = `タブを閉じる (${MOD}+W)`;
    close.textContent = '✕';
    close.tabIndex = -1;
    tabEl.append(name, close);
    el.tabstrip.appendChild(tabEl);

    /** @type {Tab} */
    const tab = { path: doc.path, name: doc.name, dir: doc.dir, encoding: doc.encoding, pane, tabEl, scrollTop: 0 };
    tabEl.addEventListener('click', () => activateTab(tab));
    tabEl.addEventListener('mousedown', (e) => { if (e.button === 1) e.preventDefault(); });
    tabEl.addEventListener('auxclick', (e) => {
      if (e.button === 1) {
        e.preventDefault();
        closeTab(tab);
      }
    });
    close.addEventListener('click', (e) => {
      e.stopPropagation();
      closeTab(tab);
    });
    return tab;
  }

  function fillPane(tab, doc, preserveScroll) {
    const article = tab.pane.firstElementChild;
    const y = preserveScroll ? (tab.pane.hidden ? tab.scrollTop : tab.pane.scrollTop) : 0;
    article.innerHTML = doc.html;
    tab.encoding = doc.encoding;
    state.imageVersion += 1;
    fixImages(article, tab.dir);
    highlightCode(article);
    tab.scrollTop = y;
    if (!tab.pane.hidden) tab.pane.scrollTop = y;
    if (tab === state.active) {
      updateChrome();
      reapplySearch();
    }
  }

  function activateTab(tab) {
    if (state.active && state.active !== tab) state.active.scrollTop = state.active.pane.scrollTop;
    state.active = tab;
    el.welcome.hidden = tab !== null;
    for (const t of state.tabs) {
      const on = t === tab;
      t.pane.hidden = !on;
      t.tabEl.classList.toggle('active', on);
      t.tabEl.setAttribute('aria-selected', String(on));
    }
    if (tab) {
      // display:none の間は scrollTop が 0 に戻るので、表示してから復元する
      tab.pane.scrollTop = tab.scrollTop;
      tab.tabEl.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
    activePane().focus({ preventScroll: true });
    updateChrome();
    reapplySearch();
  }

  function closeTab(tab) {
    const idx = state.tabs.indexOf(tab);
    if (idx < 0) return;
    state.tabs.splice(idx, 1);
    tab.pane.remove();
    tab.tabEl.remove();
    invoke('close_document', { path: tab.path }).catch(() => {});
    if (state.active === tab) {
      state.active = null;
      activateTab(state.tabs.length ? state.tabs[Math.min(idx, state.tabs.length - 1)] : null);
    } else {
      updateChrome();
    }
  }

  function cycleTab(backward) {
    if (state.tabs.length === 0) return;
    const idx = state.active ? state.tabs.indexOf(state.active) : 0;
    const n = state.tabs.length;
    activateTab(state.tabs[backward ? (idx - 1 + n) % n : (idx + 1) % n]);
  }

  function updateChrome() {
    el.tabstrip.hidden = state.tabs.length === 0;
    const tab = state.active;
    appWindow.setTitle(tab ? `${tab.name} - Markdown Viewer` : 'Markdown Viewer').catch(() => {});
    el.statusText.textContent = tab ? tab.path : HINT;
    el.statusText.title = tab ? tab.path : '';
    el.statusEncoding.textContent = tab && tab.encoding ? tab.encoding : '';
    updateZoomStatus();
  }

  // ---- ファイルを開く / 再読み込み --------------------------------------------------

  async function openFile(path) {
    const existing = findTab(path);
    if (existing) {
      activateTab(existing);
      return;
    }
    let doc;
    try {
      doc = await invoke('open_document', { path });
    } catch (err) {
      await showError(`ファイルを開けませんでした。\n${path}\n\n${err}`);
      return;
    }
    const duplicate = findTab(doc.path);
    if (duplicate) {
      activateTab(duplicate);
      return;
    }
    const tab = createTab(doc);
    state.tabs.push(tab);
    fillPane(tab, doc, false);
    activateTab(tab);
  }

  async function openFiles(paths) {
    for (const p of paths || []) await openFile(p);
  }

  async function showOpenDialog() {
    let selected;
    try {
      selected = await dialog.open({
        multiple: true,
        title: 'Markdown ファイルを開く',
        defaultPath: state.active ? state.active.dir : undefined,
        filters: [
          { name: 'Markdown ファイル', extensions: ['md', 'markdown', 'mdown', 'mkd', 'mdtxt'] },
          { name: 'テキスト ファイル', extensions: ['txt'] },
          { name: 'すべてのファイル', extensions: ['*'] },
        ],
      });
    } catch (err) {
      await showError(String(err));
      return;
    }
    if (!selected) return;
    await openFiles(Array.isArray(selected) ? selected : [selected]);
  }

  async function showError(message) {
    try {
      await dialog.message(message, { title: 'Markdown Viewer - エラー', kind: 'error' });
    } catch {
      el.statusText.textContent = message;
    }
  }

  async function reloadTab(tab) {
    try {
      const doc = await invoke('reload_document', { path: tab.path });
      fillPane(tab, doc, true);
    } catch (err) {
      el.statusText.textContent = String(err);
    }
  }

  function reloadActive() {
    if (state.active) reloadTab(state.active);
    else renderWelcome();
  }

  async function renderWelcome() {
    try {
      const html = await invoke('render_text', { markdown: WELCOME_MD });
      const article = el.welcome.firstElementChild;
      article.innerHTML = html;
      highlightCode(article);
      if (!state.active) reapplySearch();
    } catch {
      /* ウェルカム画面の描画失敗は致命的ではない */
    }
  }

  // ---- 本文の後処理: 相対画像・構文ハイライト・リンク ------------------------------------

  function fixImages(article, dir) {
    for (const img of article.querySelectorAll('img[src]')) {
      const raw = (img.getAttribute('src') || '').trim();
      if (!isRelativeUrl(raw)) continue;
      let rel = raw.split(/[?#]/)[0];
      try {
        rel = decodeURIComponent(rel);
      } catch {
        /* そのまま使う */
      }
      // 開いているドキュメントのフォルダー配下だけを Rust 側 (protocol.rs) が配信する
      img.src = `${convertFileSrc(joinPath(dir, rel), 'mdfile')}?v=${state.imageVersion}`;
    }
  }

  function highlightCode(article) {
    if (!window.hljs) return;
    for (const code of article.querySelectorAll('pre > code')) {
      if (code.textContent.length > 100000) continue;
      try {
        window.hljs.highlightElement(code);
      } catch {
        /* 未知の言語などは無視 */
      }
    }
  }
  if (window.hljs) window.hljs.configure({ ignoreUnescapedHTML: true, throwUnescapedHTML: false });

  el.content.addEventListener('click', (e) => {
    const anchor = e.target.closest('a[href]');
    if (!anchor) return;
    e.preventDefault();
    const href = (anchor.getAttribute('href') || '').trim();
    if (!href) return;
    if (href.startsWith('#')) {
      scrollToAnchor(href.slice(1));
      return;
    }
    if (/^(https?:|mailto:)/i.test(href)) {
      invoke('open_external', { url: href }).catch(() => {});
      return;
    }
    if (!isRelativeUrl(href)) return;
    const tab = state.active;
    if (!tab) return;
    invoke('resolve_link', { dir: tab.dir, href })
      .then((target) => { if (target) openFile(target); })
      .catch(() => {});
  });
  el.content.addEventListener('auxclick', (e) => {
    if (e.target.closest('a[href]')) e.preventDefault();
  });

  function scrollToAnchor(fragment) {
    let id = fragment;
    try {
      id = decodeURIComponent(fragment);
    } catch {
      /* そのまま使う */
    }
    const pane = activePane();
    const target =
      pane.querySelector(`[id="${CSS.escape(id)}"]`) || pane.querySelector(`a[name="${CSS.escape(id)}"]`);
    if (target) target.scrollIntoView({ block: 'start' });
  }

  // ---- ページ内検索 (実処理は search.js の MdSearch) -------------------------------------

  const MdSearch = window.MdSearch;
  const isSearchOpen = () => !el.searchbar.hidden;

  function openSearch() {
    el.searchbar.hidden = false;
    el.btnSearch.setAttribute('aria-pressed', 'true');
    el.searchInput.focus();
    el.searchInput.select();
    if (el.searchInput.value) runSearch('find');
  }

  function closeSearch() {
    if (!isSearchOpen()) return;
    el.searchbar.hidden = true;
    el.btnSearch.setAttribute('aria-pressed', 'false');
    el.searchCount.textContent = '';
    MdSearch.clear();
    activePane().focus({ preventScroll: true });
  }

  function toggleSearch() {
    if (isSearchOpen()) closeSearch();
    else openSearch();
  }

  /** @param {'find'|'next'|'prev'} action */
  function runSearch(action) {
    if (!isSearchOpen()) return;
    const query = el.searchInput.value;
    MdSearch.setRoot(activePane());
    if (!query) {
      MdSearch.clear();
      el.searchCount.textContent = '';
      return;
    }
    const r = action === 'find' ? MdSearch.find(query) : action === 'next' ? MdSearch.next() : MdSearch.prev();
    el.searchCount.textContent = r.count === 0 ? '一致なし' : `${r.current} / ${r.count}${r.capped ? '+' : ''}`;
  }

  /** ページの再描画やタブ切り替えの後、開いている検索を表示中のページへ適用し直す。 */
  function reapplySearch() {
    if (!isSearchOpen()) return;
    runSearch('find');
  }

  el.searchInput.addEventListener('input', () => runSearch('find'));
  el.searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      runSearch(e.shiftKey ? 'prev' : 'next');
    }
  });
  el.searchPrev.addEventListener('click', () => runSearch('prev'));
  el.searchNext.addEventListener('click', () => runSearch('next'));
  el.searchClose.addEventListener('click', closeSearch);

  // ---- ズーム ------------------------------------------------------------------

  function setZoom(value, save = true) {
    const z = Math.min(ZOOM.max, Math.max(ZOOM.min, Math.round(value / ZOOM.step) * ZOOM.step));
    state.settings.zoom = z;
    document.documentElement.style.setProperty('--zoom', String(z / 100));
    updateZoomStatus();
    if (save) scheduleSave();
  }

  function updateZoomStatus() {
    el.statusZoom.textContent = state.settings.zoom === 100 ? '' : `${state.settings.zoom}%`;
  }

  // ---- 配色 ------------------------------------------------------------------

  function buildThemeSelect() {
    for (const theme of THEMES) {
      const option = document.createElement('option');
      option.value = theme.id;
      option.textContent = theme.name;
      el.themeSelect.appendChild(option);
    }
  }

  function applyTheme(id, save = true) {
    const theme = THEMES.find((t) => t.id === id) || THEMES[0];
    document.documentElement.dataset.theme = theme.id;
    el.themeSelect.value = theme.id;
    state.settings.theme = theme.id;
    const css = getComputedStyle(document.documentElement);
    invoke('set_chrome', {
      isDark: theme.dark,
      bg: css.getPropertyValue('--window-bg').trim(),
      fg: css.getPropertyValue('--window-fg').trim(),
    }).catch(() => {});
    if (save) scheduleSave();
  }

  function scheduleSave() {
    clearTimeout(state.saveTimer);
    state.saveTimer = setTimeout(() => {
      invoke('save_settings', { settings: { ...state.settings } }).catch(() => {});
    }, 300);
  }

  // ---- ショートカット ----------------------------------------------------------------

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (isSearchOpen()) {
        e.preventDefault();
        closeSearch();
      }
      return;
    }
    if (e.key === 'F5') {
      e.preventDefault();
      reloadActive();
      return;
    }
    if (e.key === 'F3') {
      e.preventDefault();
      if (isSearchOpen() && el.searchInput.value) runSearch(e.shiftKey ? 'prev' : 'next');
      else openSearch();
      return;
    }
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      cycleTab(e.shiftKey);
      return;
    }
    // macOS の Cmd 系はネイティブメニューのアクセラレーターが処理する (menu.rs → "menu-action")
    if (IS_MAC && e.metaKey) return;
    const mod = IS_MAC ? e.metaKey : e.ctrlKey;
    if (!mod || e.altKey) return;

    const key = e.key.toLowerCase();
    if (key === 'o' && !e.shiftKey) {
      e.preventDefault();
      showOpenDialog();
    } else if (key === 'f' && !e.shiftKey) {
      e.preventDefault();
      openSearch();
    } else if (key === 'w' && !e.shiftKey) {
      e.preventDefault();
      if (state.active) closeTab(state.active);
    } else if (key === 'r' && !e.shiftKey) {
      e.preventDefault();
      reloadActive();
    } else if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd') {
      e.preventDefault();
      setZoom(state.settings.zoom + ZOOM.step);
    } else if (e.key === '-' || e.code === 'NumpadSubtract') {
      e.preventDefault();
      setZoom(state.settings.zoom - ZOOM.step);
    } else if (e.key === '0' || e.code === 'Numpad0') {
      e.preventDefault();
      setZoom(100);
    } else if (['p', 's', 'u', 'j', 'h', 'n', 't', 'g'].includes(key)) {
      // WebView 自身の印刷・保存・履歴などのショートカットを抑止する
      e.preventDefault();
    }
  });

  window.addEventListener(
    'wheel',
    (e) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      setZoom(state.settings.zoom + (e.deltaY < 0 ? ZOOM.step : -ZOOM.step));
    },
    { passive: false },
  );

  // ---- ドラッグ＆ドロップ (ウィンドウ全面。Tauri がネイティブで受けてパスを渡す) ---------------

  function showDrop(on) {
    el.dropOverlay.hidden = !on;
  }
  for (const type of ['dragover', 'drop']) document.addEventListener(type, (e) => e.preventDefault());

  // ---- macOS メニューからの操作 ----------------------------------------------------

  const MENU_ACTIONS = {
    open: showOpenDialog,
    'close-tab': () => { if (state.active) closeTab(state.active); },
    find: openSearch,
    'find-next': () => runSearch('next'),
    'find-prev': () => runSearch('prev'),
    reload: reloadActive,
    'zoom-in': () => setZoom(state.settings.zoom + ZOOM.step),
    'zoom-out': () => setZoom(state.settings.zoom - ZOOM.step),
    'zoom-reset': () => setZoom(100),
    'next-tab': () => cycleTab(false),
    'prev-tab': () => cycleTab(true),
  };

  // ---- 初期化 ------------------------------------------------------------------

  function wireToolbar() {
    el.btnOpen.title = `Markdown ファイルを開く (${MOD}+O)`;
    el.btnReload.title = IS_MAC ? '再読み込み (Cmd+R)' : '再読み込み (F5)';
    el.btnSearch.title = `ページ内検索 (${MOD}+F)`;
    el.btnOpen.addEventListener('click', showOpenDialog);
    el.btnReload.addEventListener('click', reloadActive);
    el.btnSearch.addEventListener('click', toggleSearch);
    el.themeSelect.addEventListener('change', () => applyTheme(el.themeSelect.value));
  }

  async function init() {
    buildThemeSelect();
    wireToolbar();

    try {
      Object.assign(state.settings, await invoke('load_settings'));
    } catch {
      /* 既定値で続行 */
    }
    applyTheme(state.settings.theme, false);
    setZoom(state.settings.zoom, false);
    await renderWelcome();

    await listen('open-files', ({ payload }) => openFiles(payload.paths));
    await listen('doc-changed', ({ payload }) => {
      const tab = findTab(payload.path);
      if (tab) reloadTab(tab);
    });
    await listen('menu-action', ({ payload }) => {
      const action = MENU_ACTIONS[payload.id];
      if (action) action();
    });
    await webview.onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter' || p.type === 'over') showDrop(true);
      else if (p.type === 'leave') showDrop(false);
      else if (p.type === 'drop') {
        showDrop(false);
        openFiles(p.paths);
      }
    });

    updateChrome();
    // リスナー登録後に起動ファイルを受け取る (以後の追加ファイルは open-files イベントで届く)
    const files = await invoke('take_startup_files');
    await openFiles(files);
  }

  init().catch((err) => {
    el.statusText.textContent = `初期化に失敗しました: ${err}`;
  });
})();
