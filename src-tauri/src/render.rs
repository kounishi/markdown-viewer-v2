//! Markdown → HTML 変換 (pulldown-cmark)。
//! 見出しに GitHub 流の id を付与し、<script> を含む生 HTML はテキストとして無害化する。

use std::collections::HashMap;

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};

pub fn render(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_DEFINITION_LIST);
    options.insert(Options::ENABLE_SUPERSCRIPT);
    options.insert(Options::ENABLE_SUBSCRIPT);

    let mut events: Vec<Event> = Parser::new_ext(markdown, options).collect();
    assign_heading_ids(&mut events);
    neutralize_scripts(&mut events);

    let mut out = String::with_capacity(markdown.len() * 3 / 2 + 1024);
    html::push_html(&mut out, events.into_iter());
    out
}

/// 明示 id の無い見出しへ、見出しテキストから生成した一意な id を付ける。
fn assign_heading_ids(events: &mut [Event]) {
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < events.len() {
        let explicit = match &events[i] {
            Event::Start(Tag::Heading { id, .. }) => Some(id.clone()),
            _ => None,
        };
        let Some(explicit) = explicit else {
            i += 1;
            continue;
        };

        let mut text = String::new();
        let mut j = i + 1;
        while j < events.len() {
            match &events[j] {
                Event::End(TagEnd::Heading(_)) => break,
                Event::Text(t) | Event::Code(t) => text.push_str(t),
                _ => {}
            }
            j += 1;
        }

        let id = match explicit {
            Some(id) => {
                used.insert(id.to_string(), 0);
                id.to_string()
            }
            None => unique_slug(&mut used, slugify(&text)),
        };
        if let Event::Start(Tag::Heading { id: slot, .. }) = &mut events[i] {
            *slot = Some(CowStr::from(id));
        }
        i = j.max(i + 1);
    }
}

/// GitHub 互換の slug: 小文字化、英数字・`_`・`-` 以外を除去、空白は `-`。
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    for c in text.trim().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            slug.extend(c.to_lowercase());
        } else if c.is_whitespace() {
            slug.push('-');
        }
    }
    if slug.is_empty() {
        "section".to_owned()
    } else {
        slug
    }
}

fn unique_slug(used: &mut HashMap<String, usize>, base: String) -> String {
    if !used.contains_key(&base) {
        used.insert(base.clone(), 0);
        return base;
    }
    let mut n = used[&base];
    loop {
        n += 1;
        let candidate = format!("{base}-{n}");
        if !used.contains_key(&candidate) {
            used.insert(base, n);
            used.insert(candidate.clone(), 0);
            return candidate;
        }
    }
}

/// `<script` を含む生 HTML チャンクは丸ごとテキスト扱いにしてエスケープさせる (CSP と二重の防御)。
fn neutralize_scripts(events: &mut [Event]) {
    for event in events.iter_mut() {
        let dangerous = match event {
            Event::Html(h) | Event::InlineHtml(h) => contains_script(h),
            _ => false,
        };
        if dangerous {
            let placeholder = Event::Text(CowStr::Borrowed(""));
            let raw = match std::mem::replace(event, placeholder) {
                Event::Html(h) | Event::InlineHtml(h) => h,
                _ => unreachable!(),
            };
            *event = Event::Text(raw);
        }
    }
}

fn contains_script(html: &str) -> bool {
    html.to_ascii_lowercase().contains("<script")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_gets_slug_id() {
        let html = render("# Hello World\n\n## 日本語 見出し!");
        assert!(html.contains(r#"<h1 id="hello-world">"#), "{html}");
        assert!(html.contains(r#"<h2 id="日本語-見出し">"#), "{html}");
    }

    #[test]
    fn duplicate_headings_are_deduplicated() {
        let html = render("# A\n\n# A\n\n# A");
        assert!(html.contains(r#"id="a""#));
        assert!(html.contains(r#"id="a-1""#));
        assert!(html.contains(r#"id="a-2""#));
    }

    #[test]
    fn explicit_heading_id_is_kept() {
        let html = render("# Title {#custom}");
        assert!(html.contains(r#"<h1 id="custom">"#), "{html}");
    }

    #[test]
    fn script_blocks_are_escaped() {
        let html = render("text\n\n<script>alert(1)</script>\n\nmore <script>x()</script> inline");
        assert!(!html.contains("<script"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn harmless_html_passes_through() {
        let html = render("a <br> b\n\n<details><summary>s</summary>body</details>");
        assert!(html.contains("<br>"));
        assert!(html.contains("<details>"));
    }

    #[test]
    fn gfm_features_render() {
        let html = render(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n\n~~del~~\n\n> [!NOTE]\n> note body",
        );
        assert!(html.contains("<table>"));
        assert!(html.contains(r#"type="checkbox""#));
        assert!(html.contains("<del>"));
        assert!(html.contains(r#"<blockquote class="markdown-alert-note">"#), "{html}");
    }

    #[test]
    fn superscript_and_subscript_between_words() {
        // pulldown-cmark は `_` と同じく語中 (H~2~O) では区切りとみなさない
        let html = render("a ^x^ b ~y~ c");
        assert!(html.contains("<sup>x</sup>"), "{html}");
        assert!(html.contains("<sub>y</sub>"), "{html}");
    }

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify("  Hello,  World! "), "hello--world");
        assert_eq!(slugify("C# & .NET"), "c--net");
        assert_eq!(slugify("***"), "section");
    }
}
