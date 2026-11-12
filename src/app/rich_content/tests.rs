//! 富文本内容解析的无头验收（T013 链接部分）。
//!
//! 注意：只显式导入本模块所需的公开项，不能用 `use super::*;`，否则会把父模块
//! 经 `gpui_kit::*` 引入的 gpui `test` 宏一并拉入作用域，遮蔽内置 `#[test]` 导致
//! 宏展开递归。

use super::{RichLink, RichSegment, extract_links, parse_rich};

#[test]
fn extract_links_finds_markdown_and_bare() {
    let text = "见 [官网](https://example.com) 与 http://a.com 结尾";
    let links = extract_links(text);
    assert_eq!(links.len(), 2, "应识别两个链接");
    assert_eq!(links[0].label, "官网");
    assert_eq!(links[0].url, "https://example.com");
    assert_eq!(links[1].label, "http://a.com");
    assert_eq!(links[1].url, "http://a.com");
}

#[test]
fn parse_rich_preserves_surrounding_text() {
    let segs = parse_rich("说 [x](https://y.com) 完");
    assert_eq!(segs.len(), 3);
    assert_eq!(segs[0], RichSegment::Text("说 ".to_string()));
    assert_eq!(
        segs[1],
        RichSegment::Link(RichLink {
            label: "x".to_string(),
            url: "https://y.com".to_string()
        })
    );
    assert_eq!(segs[2], RichSegment::Text(" 完".to_string()));
}

#[test]
fn extract_links_dedupes_by_url() {
    let text = "https://dup.com 与 https://dup.com 重复";
    let links = extract_links(text);
    assert_eq!(links.len(), 1, "相同地址应去重");
}

#[test]
fn parse_rich_handles_plain_text() {
    assert_eq!(
        parse_rich("纯文字无链接"),
        vec![RichSegment::Text("纯文字无链接".to_string())]
    );
}

#[test]
fn parse_rich_ignores_lone_bracket() {
    assert_eq!(
        parse_rich("数组 [1, 2] 不是链接"),
        vec![RichSegment::Text("数组 [1, 2] 不是链接".to_string())]
    );
}
