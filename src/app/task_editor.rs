//! 任务内容对话框（Phase 3 / T011）。
//!
//! 以独立 gpui 视图渲染一个居中的任务编辑面板：可编辑名称、预计耗时（分钟）、
//! 富纯文本内容。已有媒体附件以 [`Attachment`] 卡片只读渲染——图片内联预览、
//! 音频/视频提供「打开」按钮（媒体的增删为 T011b，单独提交）。频率当前只读展示，
//! 其编辑走独立的频率对话框（T014）。保存时把字段写回 `Store`，并通知根视图
//! `reload()` 后关闭本对话框。

use std::path::Path;
use std::sync::Arc;

use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::component::attachment::{
    Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
    AttachmentTitle,
};
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::app::rich_content::render_rich_text;
use crate::model::{Frequency, Task, TaskId, TaskMedia, TaskMediaKind};
use crate::store::Store;

/// 任务内容编辑器视图。
pub struct TaskEditor {
    task_id: TaskId,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    name: Entity<InputState>,
    estimated_minutes: Entity<InputState>,
    description: Entity<TextareaState>,
    frequency: Frequency,
    media: Vec<TaskMedia>,
}

impl TaskEditor {
    /// 构造编辑器；用任务当前值初始化各输入状态。
    pub fn new(
        task: &Task,
        store: Arc<Store>,
        root: Entity<MinActionApp>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let name = cx.new(|c| InputState::new(window, c).default_value(task.name.as_str()));
        let description = cx.new(|c| {
            TextareaState::new(window, c).default_value(task.description.as_deref().unwrap_or(""))
        });
        let est = task
            .estimated_minutes
            .map(|m| m.to_string())
            .unwrap_or_default();
        let estimated_minutes = cx.new(|c| InputState::new(window, c).default_value(est.as_str()));
        let media = store.task_media(task.id).unwrap_or_default();
        Self {
            task_id: task.id,
            store,
            root,
            name,
            estimated_minutes,
            description,
            frequency: task.frequency,
            media,
        }
    }
}

impl Render for TaskEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let task_id = self.task_id;
        let root = self.root.clone();
        let name_state = self.name.clone();
        let est_state = self.estimated_minutes.clone();
        let desc_state = self.description.clone();
        let store = self.store.clone();
        let frequency_label = self.frequency.label();
        let media = self.media.clone();
        let media_dir = self
            .store
            .data_dir()
            .join("task_content")
            .join(task_id.0.to_string());
        let desc_preview = self.description.read(cx).value().to_string();

        div()
            .id("task-editor")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(560.))
            .max_h(px(680.))
            .overflow_y_scroll()
            .child(div().text_size(px(18.)).child("任务内容"))
            .child(field("名称", Input::new(&name_state)))
            .child(field("预计耗时（分钟）", Input::new(&est_state)))
            .child(field("内容", Textarea::new(&desc_state)))
            .when(!desc_preview.trim().is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_size(px(12.)).child("内容预览（点击链接打开）"))
                        .child(render_rich_text(&desc_preview)),
                )
            })
            .child(div().child(format!("频率：{frequency_label}")))
            .child(media_block(&media, &media_dir))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(save_button(
                        task_id, name_state, est_state, desc_state, store, root,
                    )),
            )
    }
}

/// 一个「标签 + 输入控件」的纵向组合。
fn field(label: &'static str, input: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(px(12.)).child(label))
        .child(input)
}

/// 只读展示已有媒体附件：图片内联预览，音频/视频提供「打开」按钮。
///
/// 媒体文件位于 `<data_dir>/task_content/<task_id>/<file_name>`，由 `media_dir` 给出目录。
fn media_block(media: &[TaskMedia], media_dir: &Path) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_size(px(12.)).child("媒体"))
        .children(media.iter().enumerate().map(|(idx, m)| {
            let path = media_dir.join(&m.file_name);
            let open_uri = file_uri(&path);
            let is_image = matches!(m.kind, TaskMediaKind::Image);
            let card = Attachment::new()
                .id(("media", idx))
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(m.original_name.clone()))
                        .description(AttachmentDescription::new(format!("{:?}", m.kind))),
                )
                .actions(AttachmentActions::new().child(
                    Button::new(("open-media", idx)).label("打开").on_click(
                        move |_ev, _window, cx| {
                            cx.open_url(&open_uri);
                        },
                    ),
                ));
            if is_image {
                card.axis(gpui::Axis::Vertical)
                    .media(AttachmentMedia::new().src(path))
            } else {
                card
            }
        }))
}

/// 把本地路径转成 `file://` URI，交给系统默认程序打开。
fn file_uri(path: &Path) -> String {
    format!("file:///{}", path.display().to_string().replace('\\', "/"))
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("editor-cancel")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .child("取消")
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |root, cx| {
                root.close_editor();
                cx.notify();
            });
        })
}

/// 「保存」按钮：写回数据后关闭。
fn save_button(
    task_id: TaskId,
    name_state: Entity<InputState>,
    est_state: Entity<InputState>,
    desc_state: Entity<TextareaState>,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
) -> impl IntoElement {
    div()
        .id("editor-save")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(rgb(0x33_33_38))
        .child("保存")
        .on_click(move |_ev, _window, cx| {
            let name = name_state.read(cx).value().trim().to_string();
            let description = desc_state.read(cx).value().trim().to_string();
            let estimated = est_state.read(cx).value().trim().parse::<u16>().ok();
            let _ = apply_edits(&store, task_id, &name, &description, estimated);
            root.update(cx, |root, cx| {
                let _ = root.reload();
                root.close_editor();
                cx.notify();
            });
        })
}

/// 把任务编辑结果写回 `Store`：重命名、设内容、设预计耗时。
///
/// 抽成自由函数，便于在无 GPU 的单元/集成测试中直接验证写回逻辑。
pub(crate) fn apply_edits(
    store: &Store,
    task_id: TaskId,
    name: &str,
    description: &str,
    estimated_minutes: Option<u16>,
) -> anyhow::Result<()> {
    store.rename_task(task_id, name)?;
    store.set_task_description(task_id, description)?;
    store.update_task_estimated_minutes(task_id, estimated_minutes)?;
    Ok(())
}

#[cfg(test)]
mod tests;
