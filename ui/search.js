/* ページ内検索。
 * 参考アプリ MarkdownPage.cs の window.__mdSearch を移植し、走査とスクロールをアクティブなタブの pane に限定した。
 * DOM を書き換えない CSS Custom Highlight API を使い、非対応環境 (古い WKWebView) では <mark> で包むフォールバックに切り替える。
 * find/next/prev/clear の戻り値はすべて { count, current, capped } (current は 1 始まり、0 なら該当なし)。 */
window.MdSearch = (() => {
  'use strict';

  const NAME = 'md-search';
  const CURRENT = 'md-search-current';
  const LIMIT = 10000;
  const supported = typeof CSS !== 'undefined' && 'highlights' in CSS && typeof Highlight === 'function';

  let root = null;
  let ranges = [];
  let index = -1;
  let capped = false;
  /** フォールバック時のみ: 一致ごとの <mark> 要素の配列 */
  let marks = [];

  const result = () => ({ count: ranges.length, current: index + 1, capped });

  function setRoot(element) {
    if (element !== root) {
      clear();
      root = element;
    }
  }

  function clear() {
    if (supported) {
      CSS.highlights.delete(NAME);
      CSS.highlights.delete(CURRENT);
    } else {
      unwrapMarks();
    }
    ranges = [];
    index = -1;
    capped = false;
    return result();
  }

  // 本文のテキストノードを連結し、各ノードの開始オフセットを記録する
  function collectText() {
    const nodes = [];
    const starts = [];
    let text = '';
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode(node) {
        const parent = node.parentElement;
        if (!parent || parent.closest('script,style,noscript')) return NodeFilter.FILTER_REJECT;
        // ブロック要素の間にある整形用の改行 (画面には表示されない) は対象外
        if (!/\S/.test(node.data) && node.data.includes('\n')) return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    let node;
    while ((node = walker.nextNode())) {
      nodes.push(node);
      starts.push(text.length);
      text += node.data;
    }
    return { nodes, starts, text };
  }

  // 連結テキスト上のオフセットを含むノードの番号 (二分探索)
  function locate(starts, offset) {
    let lo = 0;
    let hi = starts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (starts[mid] <= offset) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  }

  // 大文字小文字を区別せず、語句内の空白は改行を含む任意の空白列に一致させる
  function buildRegex(query) {
    const parts = query
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .map((s) => s.replace(/[.*+?^$()|[\]\\{}]/g, '\\$&'));
    if (parts.length === 0) return null;
    return new RegExp(parts.join('\\s+'), 'giu');
  }

  function find(query) {
    clear();
    if (!root || !query) return result();
    const re = buildRegex(query);
    if (!re) return result();

    const { nodes, starts, text } = collectText();
    if (nodes.length === 0) return result();

    let m;
    while ((m = re.exec(text)) !== null) {
      if (m[0].length === 0) {
        re.lastIndex++;
        continue;
      }
      if (ranges.length >= LIMIT) {
        capped = true;
        break;
      }
      const s = m.index;
      const e = s + m[0].length;
      const i = locate(starts, s);
      const j = locate(starts, e - 1);
      const range = new Range();
      range.setStart(nodes[i], s - starts[i]);
      range.setEnd(nodes[j], e - starts[j]);
      ranges.push(range);
    }
    if (ranges.length === 0) return result();

    if (supported) {
      const all = new Highlight();
      for (const range of ranges) all.add(range);
      CSS.highlights.set(NAME, all);
    } else {
      wrapMarks(nodes);
    }

    // 現在の表示位置以降で最初の一致をカレントにする (ブラウザーの Ctrl+F と同じ挙動)
    const top = root.getBoundingClientRect().top;
    index = ranges.findIndex((_, k) => rectOf(k).bottom >= top);
    if (index < 0) index = 0;
    setCurrent();
    return result();
  }

  function rectOf(k) {
    if (supported) return ranges[k].getBoundingClientRect();
    let rect = null;
    for (const mark of marks[k]) {
      const b = mark.getBoundingClientRect();
      rect = rect
        ? { top: Math.min(rect.top, b.top), bottom: Math.max(rect.bottom, b.bottom) }
        : { top: b.top, bottom: b.bottom };
    }
    return rect || { top: 0, bottom: 0 };
  }

  function setCurrent() {
    if (supported) {
      CSS.highlights.set(CURRENT, new Highlight(ranges[index]));
    } else {
      marks.forEach((els, k) => els.forEach((mark) => mark.classList.toggle('current', k === index)));
    }
    const rect = rectOf(index);
    const rootRect = root.getBoundingClientRect();
    if (rect.top < rootRect.top || rect.bottom > rootRect.bottom) {
      const height = rect.bottom - rect.top;
      root.scrollTop += rect.top - rootRect.top - (root.clientHeight - height) / 2;
    }
  }

  function step(delta) {
    if (ranges.length === 0) return result();
    index = (index + delta + ranges.length) % ranges.length;
    setCurrent();
    return result();
  }

  // ---- フォールバック: 一致箇所をテキストノード単位で <mark> に包む ----
  function wrapMarks(nodes) {
    marks = ranges.map(() => []);
    // 後ろの一致から処理すると、テキストノードの分割で前方の一致のオフセットが崩れない
    for (let k = ranges.length - 1; k >= 0; k--) {
      const range = ranges[k];
      const i = nodes.indexOf(range.startContainer);
      const j = nodes.indexOf(range.endContainer);
      if (i < 0 || j < 0) continue;
      for (let n = j; n >= i; n--) {
        const node = nodes[n];
        const from = n === i ? range.startOffset : 0;
        const to = n === j ? range.endOffset : node.data.length;
        if (to <= from) continue;
        const segment = new Range();
        segment.setStart(node, from);
        segment.setEnd(node, to);
        const mark = document.createElement('mark');
        mark.className = 'md-mark';
        try {
          segment.surroundContents(mark);
          marks[k].unshift(mark);
        } catch {
          /* 要素境界をまたぐ異常ケースは飛ばす */
        }
      }
    }
  }

  function unwrapMarks() {
    if (marks.length === 0) return;
    const parents = new Set();
    for (const els of marks) {
      for (const mark of els) {
        if (!mark.parentNode) continue;
        parents.add(mark.parentNode);
        mark.replaceWith(...mark.childNodes);
      }
    }
    for (const parent of parents) parent.normalize();
    marks = [];
  }

  return { setRoot, find, next: () => step(1), prev: () => step(-1), clear, supported };
})();
