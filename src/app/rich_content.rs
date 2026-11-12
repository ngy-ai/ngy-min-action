//! 富文本内容渲染（T013）：把任务描述中的链接渲染为可点击元素。
//!
//! 纯解析逻辑 `parse_rich` / `extract_links` 不依赖 GPU，可在单元测试中验证；
//! `render_rich_text` 用换行布局把链接渲染为可点击文字，点击经 `cx.open_url`
//! 在系统浏览器打开。图片内联与音视频「打开」依赖 `Store` 暴露媒体文件绝对路径
//! （归 T011b 媒体增删），本模块先只覆盖「链接可点击」。

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 一段被识别出的链接。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichLink {
    /// 展示文字：markdown 标签，或裸链接本身。
    pub label: String,
    /// 链接地址（已含协议）。
    pub url: String,
}

/// 富文本的一个片段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RichSegment {
    /// 普通文字。
    Text(String),
    /// 可点击链接。
    Link(RichLink),
}

/// 解析文本为富片段：支持 markdown `[label](url)` 与裸 `http(s)://` 地址。
pub fn parse_rich(text: &str) -> Vec<RichSegment> {
    let mut out = Vec::new();
    let n = text.len();
    let mut cursor = 0;
    while cursor < n {
        if let Some((range, link)) = try_markdown_at(text, cursor) {
            if range.start > cursor {
                out.push(RichSegment::Text(text[cursor..range.start].to_string()));
            }
            out.push(RichSegment::Link(link));
            cursor = range.end;
            continue;
        }
        if text[cursor..].starts_with("http://") || text[cursor..].starts_with("https://") {
            let end = text[cursor..]
                .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
                .map(|d| cursor + d)
                .unwrap_or(n);
            let url = text[cursor..end].to_string();
            out.push(RichSegment::Link(RichLink {
                label: url.clone(),
                url,
            }));
            cursor = end;
            continue;
        }
        // 普通文字：一直累积到下一个真正的链接起点。
        let mut end = cursor + text[cursor..].chars().next().unwrap().len_utf8();
        while end < n {
            let is_link = try_markdown_at(text, end).is_some()
                || text[end..].starts_with("http://")
                || text[end..].starts_with("https://");
            if is_link {
                break;
            }
            end += text[end..].chars().next().unwrap().len_utf8();
        }
        out.push(RichSegment::Text(text[cursor..end].to_string()));
        cursor = end;
    }
    out
}

/// 仅抽出文本中的全部链接（去重保持顺序）。
pub fn extract_links(text: &str) -> Vec<RichLink> {
    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();
    for seg in parse_rich(text) {
        if let RichSegment::Link(link) = seg
            && seen.insert(link.url.clone())
        {
            links.push(link);
        }
    }
    links
}

/// 把富文本渲染为可点击链接与文字混排的元素：整段文字原样显示，
/// 抽出的链接作为可点击行单独列出（点击经 `cx.open_url` 打开）。
pub fn render_rich_text(text: &str) -> impl IntoElement {
    let links = extract_links(text);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().child(text.to_string()))
        .when(!links.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(links.into_iter().enumerate().map(|(idx, link)| {
                        let url = link.url.clone();
                        div()
                            .id(("rich-link", idx))
                            .text_color(rgb(0x33_99_ff))
                            .underline()
                            .cursor_pointer()
                            .on_click(move |_ev, _window, cx| {
                                cx.open_url(&url);
                            })
                            .child(link.label)
                    })),
            )
        })
}

/// 尝试在 `cursor` 处解析一个 markdown 链接 `[label](url)`。
fn try_markdown_at(text: &str, cursor: usize) -> Option<(std::ops::Range<usize>, RichLink)> {
    let rest = &text[cursor..];
    if !rest.starts_with('[') {
        return None;
    }
    let close = rest.find(']')?;
    let label = rest[1..close].to_string();
    if rest.get(close + 1..close + 2) != Some("(") {
        return None;
    }
    let url_start = close + 2;
    let url_end = rest[url_start..].find(')')? + url_start;
    let url = rest[url_start..url_end].to_string();
    if url.is_empty() {
        return None;
    }
    let end = url_end + 1;
    Some((cursor..cursor + end, RichLink { label, url }))
}

#[cfg(test)]
mod tests;
