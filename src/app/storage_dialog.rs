//! 存储对话框（T014）：展示数据目录与占用空间。
//!
//! 复用 `Store::data_dir` 拿到根目录后，递归累加文件字节数得到占用，并以人类可读形式展示。
//! 目录遍历与字节格式化抽成纯函数（`collect_storage` / `format_bytes`），便于无头测试。

use std::path::{Path, PathBuf};

use gpui::{Context, Entity, Render, Window};
use gpui_kit::*;

use crate::app::MinActionApp;

/// 存储信息：数据根目录与递归统计出的总占用字节数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StorageInfo {
    /// 数据根目录（含数据库与附件）。
    pub data_dir: PathBuf,
    /// 该目录下所有文件的总字节数。
    pub total_bytes: u64,
}

/// 递归累加目录树里所有文件的字节数；目录不存在时返回 0。
pub(crate) fn collect_storage(data_dir: &Path) -> StorageInfo {
    let total_bytes = sum_dir(data_dir);
    StorageInfo {
        data_dir: data_dir.to_path_buf(),
        total_bytes,
    }
}

/// 深度优先累加一个目录下的文件大小（忽略符号链接，遇不可读项跳过）。
fn sum_dir(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => total += sum_dir(&path),
            Ok(file_type) if file_type.is_file() => {
                if let Ok(meta) = std::fs::metadata(&path) {
                    total += meta.len();
                }
            }
            _ => {}
        }
    }
    total
}

/// 把字节数格式化成人类可读串（1024 进制）：`512 B`、`1.5 KB`、`2.0 MB`、`1.2 GB`。
pub(crate) fn format_bytes(bytes: u64) -> String {
    const K: f64 = 1024.0;
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let (value, unit) = {
        let kb = bytes as f64 / K;
        if kb < K {
            (kb, "KB")
        } else {
            let mb = kb / K;
            if mb < K { (mb, "MB") } else { (mb / K, "GB") }
        }
    };
    format!("{value:.1} {unit}")
}

/// 存储信息展示的「标签-值」行，供渲染与无头测试复用。
pub(crate) fn storage_rows(info: &StorageInfo) -> Vec<(&'static str, String)> {
    vec![
        ("数据目录", info.data_dir.display().to_string()),
        ("占用空间", format_bytes(info.total_bytes)),
    ]
}

/// 存储对话框视图。
pub struct StorageDialog {
    root: Entity<MinActionApp>,
    info: StorageInfo,
}

impl StorageDialog {
    /// 用已算好的存储信息构造对话框。
    pub fn new(info: StorageInfo, root: Entity<MinActionApp>) -> Self {
        Self { root, info }
    }
}

impl Render for StorageDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.clone();
        let rows = storage_rows(&self.info);
        div()
            .id("storage-dialog")
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .w(px(420.))
            .child(div().text_size(px(18.)).child("存储"))
            .children(rows.into_iter().map(|(label, value)| {
                div()
                    .flex()
                    .justify_between()
                    .child(div().child(label))
                    .child(div().child(value))
            }))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root)),
            )
    }
}

/// 「关闭」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("storage-close")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .child("关闭")
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |root, cx| {
                root.close_dialog();
                cx.notify();
            });
        })
}

#[cfg(test)]
mod tests;
