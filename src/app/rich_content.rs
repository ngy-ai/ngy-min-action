//! 任务内容的富文本渲染：自动识别链接、按类型呈现实媒体附件。

use std::path::{Path, PathBuf};

use eframe::egui;
use std::process::Command;

use crate::app::note_dialog::image_uri;
use crate::attachment::MediaStore;
use crate::model::{TaskId, TaskMedia, TaskMediaKind};

/// 富文本里的一段：普通文字或可点击的链接。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSegment {
    /// 普通文字。
    Text(String),
    /// `http(s)://` 链接，渲染为可点击并交由系统默认浏览器打开。
    Link(String),
}

/// 从纯文本里切出 `http(s)://` 链接，其余作为普通文字。
///
/// 只识别显式的 `http` / `https` 协议，不解析裸域名、不加任何 HTML 标签，避免注入面。
/// 链接一直取到第一个空白字符为止，并去掉常见的结尾标点（`.,;:)` 等），方便在句尾书写 URL。
pub fn parse_links(text: &str) -> Vec<TextSegment> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut segments = Vec::new();
    let mut buffer = String::new();
    let mut index = 0;
    while index < len {
        let slice = &text[index..];
        if slice.starts_with("http://") || slice.starts_with("https://") {
            if !buffer.is_empty() {
                segments.push(TextSegment::Text(std::mem::take(&mut buffer)));
            }
            let start = index;
            let mut end = index;
            while end < len && !bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            let mut link = text[start..end].to_owned();
            while matches!(
                link.chars().last(),
                Some('.') | Some(',') | Some(';') | Some(':') | Some(')') | Some(']') | Some('}')
            ) {
                link.pop();
            }
            if !link.is_empty() {
                segments.push(TextSegment::Link(link));
            }
            index = end;
        } else {
            let character = text[index..].chars().next().expect("字节边界切出的字符");
            buffer.push(character);
            index += character.len_utf8();
        }
    }
    if !buffer.is_empty() {
        segments.push(TextSegment::Text(buffer));
    }
    segments
}

/// 用系统默认程序打开一个本地文件（图片预览、音频/视频播放都走这里）。
///
/// Windows 下用 `cmd /c start`，macOS 用 `open`，其余用 `xdg-open`。
#[cfg(not(target_arch = "wasm32"))]
pub fn open_file(path: &Path) -> std::io::Result<()> {
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/c", "start", "", &path.to_string_lossy()])
            .status()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()
    } else {
        Command::new("xdg-open").arg(path).status()
    };
    status.map(|_| ())
}

/// 渲染一段任务内容：文字里的链接可点击，图片内联，音频/视频给出打开按钮。
pub fn render_rich_content(
    ui: &mut egui::Ui,
    description: Option<&str>,
    media: &[TaskMedia],
    media_store: &MediaStore,
    task_id: TaskId,
) {
    match description {
        Some(text) if !text.trim().is_empty() => {
            for segment in parse_links(text) {
                match segment {
                    TextSegment::Text(content) => {
                        ui.label(content);
                    }
                    TextSegment::Link(url) => {
                        ui.hyperlink(url);
                    }
                }
            }
        }
        _ => {
            ui.weak("任务没有内容");
        }
    }

    if !media.is_empty() {
        ui.separator();
        ui.weak(format!("附件（{}）", media.len()));
        for item in media {
            let path: Option<PathBuf> = media_store.path(task_id, &item.file_name).ok();
            match item.kind {
                TaskMediaKind::Image => {
                    if let Some(file) = path
                        && file.try_exists().unwrap_or(false)
                    {
                        let uri = image_uri(&file);
                        ui.add(egui::Image::new(uri).max_size(egui::vec2(240.0, 160.0)));
                        ui.weak(&item.original_name);
                        continue;
                    }
                    ui.weak(format!("图片缺失：{}", item.original_name));
                }
                TaskMediaKind::Audio | TaskMediaKind::Video => {
                    let label = match item.kind {
                        TaskMediaKind::Audio => "▶ 打开音频",
                        TaskMediaKind::Video => "▶ 打开视频",
                        TaskMediaKind::Image => unreachable!(),
                    };
                    ui.horizontal(|ui| {
                        if ui.button(label).clicked()
                            && let Some(file) = &path
                        {
                            let _ = open_file(file);
                        }
                        ui.weak(&item.original_name);
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_has_one_segment() {
        assert_eq!(
            parse_links("只是普通文字"),
            vec![TextSegment::Text("只是普通文字".to_owned())]
        );
    }

    #[test]
    fn link_is_extracted() {
        assert_eq!(
            parse_links("参考 https://example.com/guide 完成"),
            vec![
                TextSegment::Text("参考 ".to_owned()),
                TextSegment::Link("https://example.com/guide".to_owned()),
                TextSegment::Text(" 完成".to_owned()),
            ]
        );
    }

    #[test]
    fn trailing_punctuation_is_not_part_of_link() {
        assert_eq!(
            parse_links("见 https://example.com."),
            vec![
                TextSegment::Text("见 ".to_owned()),
                TextSegment::Link("https://example.com".to_owned()),
            ]
        );
    }

    #[test]
    fn multiple_links() {
        assert_eq!(
            parse_links("a https://a.com b http://b.org c"),
            vec![
                TextSegment::Text("a ".to_owned()),
                TextSegment::Link("https://a.com".to_owned()),
                TextSegment::Text(" b ".to_owned()),
                TextSegment::Link("http://b.org".to_owned()),
                TextSegment::Text(" c".to_owned()),
            ]
        );
    }

    #[test]
    fn non_http_scheme_is_treated_as_text() {
        assert_eq!(
            parse_links("file:///C:/x.png 不是链接"),
            vec![TextSegment::Text("file:///C:/x.png 不是链接".to_owned())]
        );
    }
}
