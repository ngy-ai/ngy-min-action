//! 任务内容对话框（Phase 3 / T011）。
//!
//! 以独立 gpui 视图渲染一个居中的任务编辑面板：可编辑名称、预计耗时（分钟）、
//! 富纯文本内容。媒体附件以 [`Attachment`] 卡片渲染——图片内联预览、音频/视频
//! 提供「打开」按钮，并支持「添加媒体」（经 `rfd` 选文件，拷入
//! `data_dir/task_content/<task_id>/` 后写库）与逐条「删除」（删库记录并清磁盘文件）。
//! 频率当前只读展示，其编辑走独立的频率对话框（T014）。保存时把字段写回 `Store`，
//! 并通知根视图 `reload()` 后关闭本对话框。

use std::path::Path;
use std::sync::Arc;

use crate::palette::Palette;
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
use crate::model::{Frequency, Task, TaskId, TaskMedia, TaskMediaId, TaskMediaKind};
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

    /// 由频率对话框回写新的生成频率（保存时再统一写库）。
    pub(crate) fn set_frequency(&mut self, frequency: Frequency, cx: &mut Context<Self>) {
        self.frequency = frequency;
        cx.notify();
    }
}

impl Render for TaskEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let task_id = self.task_id;
        let p = Palette::from_cx(cx);
        let root = self.root.clone();
        let name_state = self.name.clone();
        let est_state = self.estimated_minutes.clone();
        let desc_state = self.description.clone();
        let store = self.store.clone();
        let frequency_label = self.frequency.label();
        let media = self.media.clone();
        let entity = cx.entity().clone();
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().child(format!("频率：{frequency_label}")))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(Button::new("edit-frequency").label("编辑").on_click({
                                let root = root.clone();
                                let entity = entity.clone();
                                let current = self.frequency;
                                move |_ev, window, cx| {
                                    root.update(cx, |root, cx| {
                                        let app_entity = cx.entity().clone();
                                        root.open_frequency_dialog(
                                            app_entity,
                                            entity.clone(),
                                            current,
                                            window,
                                            cx,
                                        );
                                        cx.notify();
                                    });
                                }
                            }))
                            .child(Button::new("reset-progress").label("重置进度").on_click({
                                let root = root.clone();
                                let task_id = self.task_id;
                                move |_ev, window, cx| {
                                    root.update(cx, |root, cx| {
                                        let app_entity = cx.entity().clone();
                                        root.open_reset_dialog(app_entity, task_id, window, cx);
                                        cx.notify();
                                    });
                                }
                            })),
                    ),
            )
            .child(media_block(&media, &media_dir, entity))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(save_button(
                        task_id,
                        EditFields {
                            name: name_state,
                            est: est_state,
                            desc: desc_state,
                        },
                        store,
                        root,
                        self.frequency,
                        &p,
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

/// 展示并管理任务媒体附件：头部有「添加媒体」按钮，每张卡片提供「打开」与「删除」。
///
/// 媒体文件位于 `<data_dir>/task_content/<task_id>/<file_name>`，由 `media_dir` 给出目录。
/// 增删通过 `entity`（即 [`TaskEditor`] 自身）在点击回调里写库并更新 `self.media`。
fn media_block(
    media: &[TaskMedia],
    media_dir: &Path,
    entity: Entity<TaskEditor>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(12.)).child("媒体"))
                .child(Button::new("add-media").label("添加媒体").on_click({
                    let add_entity = entity.clone();
                    move |_ev, _window, cx| {
                        let picked = rfd::FileDialog::new()
                            .add_filter(
                                "媒体文件",
                                &[
                                    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "mp3",
                                    "wav", "ogg", "m4a", "mp4", "webm", "mov", "avi", "mkv",
                                ],
                            )
                            .pick_file();
                        if let Some(path) = picked {
                            add_entity.update(cx, |this, cx| {
                                if let Err(e) = add_media_to_task(this, &path) {
                                    tracing::error!("添加媒体失败：{e:#}");
                                }
                                cx.notify();
                            });
                        }
                    }
                })),
        )
        .children(media.iter().enumerate().map(|(idx, m)| {
            let path = media_dir.join(&m.file_name);
            let open_uri = file_uri(&path);
            let is_image = matches!(m.kind, TaskMediaKind::Image);
            let media_id = m.id;
            let edit_entity = entity.clone();
            let card = Attachment::new()
                .id(("media", idx))
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(m.original_name.clone()))
                        .description(AttachmentDescription::new(format!("{:?}", m.kind))),
                )
                .actions(
                    AttachmentActions::new().child(
                        div()
                            .flex()
                            .gap_2()
                            .child(Button::new(("open-media", idx)).label("打开").on_click(
                                move |_ev, _window, cx| {
                                    cx.open_url(&open_uri);
                                },
                            ))
                            .child(Button::new(("del-media", idx)).label("删除").on_click(
                                move |_ev, _window, cx| {
                                    edit_entity.update(cx, |this, cx| {
                                        remove_media_from_task(this, media_id);
                                        cx.notify();
                                    });
                                },
                            )),
                    ),
                );
            if is_image {
                card.axis(gpui::Axis::Vertical)
                    .media(AttachmentMedia::new().src(path))
            } else {
                card
            }
        }))
}

/// 由文件扩展名推断媒体类型；无法识别时按图片处理，避免既有数据被丢弃。
fn infer_kind(path: &Path) -> TaskMediaKind {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => TaskMediaKind::Image,
        "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac" | "wma" => TaskMediaKind::Audio,
        "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v" => TaskMediaKind::Video,
        _ => TaskMediaKind::Image,
    }
}

/// 把文件拷入本任务媒体目录并写库记录，返回新建的媒体记录。
///
/// 抽成自由函数，便于在无 GPU 的测试中直接验证「拷文件 + 写库」闭环。
pub(crate) fn add_media_file(
    store: &Store,
    task_id: TaskId,
    src: &Path,
) -> anyhow::Result<TaskMedia> {
    let media_dir = store
        .data_dir()
        .join("task_content")
        .join(task_id.0.to_string());
    std::fs::create_dir_all(&media_dir).map_err(|e| anyhow::anyhow!("创建媒体目录失败：{e}"))?;
    let original_name = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("媒体")
        .to_string();
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = infer_kind(src);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file_name = if ext.is_empty() {
        format!("m{nanos}")
    } else {
        format!("m{nanos}.{ext}")
    };
    std::fs::copy(src, media_dir.join(&file_name))
        .map_err(|e| anyhow::anyhow!("拷贝媒体文件失败：{e}"))?;
    let byte_size = std::fs::metadata(media_dir.join(&file_name))?.len();
    store
        .add_task_media(task_id, kind, &file_name, &original_name, byte_size)
        .map_err(|e| anyhow::anyhow!("记录媒体元数据失败：{e}"))
}

/// 删除一段媒体：先删库记录（得到文件名），再清磁盘文件。出错时记录日志。
///
/// 抽成自由函数，便于在无 GPU 的测试中直接验证「删库 + 删文件」闭环。
pub(crate) fn remove_media_file(store: &Store, task_id: TaskId, media_id: TaskMediaId) {
    match store.delete_task_media(media_id) {
        Ok(media) => {
            let path = store
                .data_dir()
                .join("task_content")
                .join(task_id.0.to_string())
                .join(&media.file_name);
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("删除媒体磁盘文件失败（记录已移除）：{path:?}：{e}");
            }
        }
        Err(e) => tracing::error!("删除媒体记录失败：{e}"),
    }
}

/// 把用户选中的文件加入本任务（写库 + 刷新列表）。
fn add_media_to_task(this: &mut TaskEditor, src: &Path) -> anyhow::Result<()> {
    add_media_file(&this.store, this.task_id, src)?;
    this.media = this.store.task_media(this.task_id).unwrap_or_default();
    Ok(())
}

/// 从本任务删除一段媒体（删库 + 删文件 + 刷新列表）。
fn remove_media_from_task(this: &mut TaskEditor, media_id: TaskMediaId) {
    remove_media_file(&this.store, this.task_id, media_id);
    this.media = this.store.task_media(this.task_id).unwrap_or_default();
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

/// 任务编辑器里的三个输入控件句柄，打包以控制函数参数数量。
struct EditFields {
    name: Entity<InputState>,
    est: Entity<InputState>,
    desc: Entity<TextareaState>,
}

/// 「保存」按钮：写回数据后关闭。
fn save_button(
    task_id: TaskId,
    fields: EditFields,
    store: Arc<Store>,
    root: Entity<MinActionApp>,
    frequency: Frequency,
    palette: &Palette,
) -> impl IntoElement {
    div()
        .id("editor-save")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(palette.raised)
        .child("保存")
        .on_click(move |_ev, _window, cx| {
            let name = fields.name.read(cx).value().trim().to_string();
            let description = fields.desc.read(cx).value().trim().to_string();
            let estimated = fields.est.read(cx).value().trim().parse::<u16>().ok();
            let _ = apply_edits(&store, task_id, &name, &description, estimated, frequency);
            root.update(cx, |root, cx| {
                let _ = root.reload();
                root.close_editor();
                cx.notify();
            });
        })
}

/// 把任务编辑结果写回 `Store`：重命名、设内容、设预计耗时、设频率。
///
/// 抽成自由函数，便于在无 GPU 的单元/集成测试中直接验证写回逻辑。
pub(crate) fn apply_edits(
    store: &Store,
    task_id: TaskId,
    name: &str,
    description: &str,
    estimated_minutes: Option<u16>,
    frequency: Frequency,
) -> anyhow::Result<()> {
    store.rename_task(task_id, name)?;
    store.set_task_description(task_id, description)?;
    store.update_task_estimated_minutes(task_id, estimated_minutes)?;
    store.update_task_frequency(task_id, frequency)?;
    Ok(())
}

#[cfg(test)]
mod tests;
