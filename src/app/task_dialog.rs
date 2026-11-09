//! 任务内容编辑弹窗：富纯文本说明 + 图片/音频/视频附件。

use std::path::PathBuf;

use eframe::egui;
use tracing::warn;

use crate::app::MinActionApp;
use crate::app::icons::{self, ButtonId};
use crate::app::note_dialog::image_uri;
use crate::attachment::MediaStore;
use crate::common::MAX_TASK_DESCRIPTION;
use crate::model::{TaskId, TaskMedia, TaskMediaId, TaskMediaKind};

/// 任务内容编辑状态。
#[derive(Debug)]
pub(crate) struct TaskDetailEditor {
    /// 正在编辑的任务。
    pub(crate) task_id: TaskId,
    /// 任务标题（只读展示）。
    pub(crate) name: String,
    /// 内容文本。
    pub(crate) description: String,
    /// 已附带的媒体。
    pub(crate) media: Vec<TaskMedia>,
    /// 操作错误。
    pub(crate) error: Option<String>,
}

impl MinActionApp {
    /// 打开某条任务的内容编辑弹窗。
    pub(crate) fn open_task_detail(&mut self, task_id: TaskId) {
        let result = (self.store.task(task_id), self.store.task_media(task_id));
        match result {
            (Ok(task), Ok(media)) => {
                self.task_detail = Some(TaskDetailEditor {
                    task_id,
                    name: task.name,
                    description: task.description.unwrap_or_default(),
                    media,
                    error: None,
                });
            }
            (Err(error), _) | (_, Err(error)) => {
                warn!(
                    op = "open_task_detail",
                    result = "error",
                    task = task_id.0,
                    error = %error,
                    "任务或其媒体读取失败"
                );
                self.error = Some(format!("打开任务内容失败：{error}"));
            }
        }
    }

    /// 任务内容弹窗。
    pub(crate) fn ui_task_dialog(&mut self, ctx: &egui::Context) {
        let Some(task_id) = self.task_detail.as_ref().map(|editor| editor.task_id) else {
            return;
        };
        let data_dir = self.data_dir.clone();
        let mut open = true;
        let mut save = false;
        let mut close = false;
        let mut add_media = false;
        let mut remove: Option<TaskMediaId> = None;

        egui::Window::new("任务内容")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let editor = self.task_detail.as_mut().expect("窗口已打开，编辑器必在");
                ui.strong(&editor.name);
                ui.separator();
                ui.label("内容：支持 https:// 链接，可附图片、音频、视频。");
                ui.add(
                    egui::TextEdit::multiline(&mut editor.description)
                        .desired_rows(6)
                        .desired_width(f32::INFINITY)
                        .char_limit(MAX_TASK_DESCRIPTION),
                );
                ui.weak(format!(
                    "{}/{} 个字符",
                    editor.description.chars().count(),
                    MAX_TASK_DESCRIPTION
                ));

                ui.separator();
                ui.label(format!("媒体（{}）", editor.media.len()));
                let media_store = MediaStore::new(&data_dir);
                ui.horizontal_wrapped(|ui| {
                    for item in &editor.media {
                        ui.allocate_ui_with_layout(
                            egui::vec2(160.0, 120.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                match item.kind {
                                    TaskMediaKind::Image => {
                                        if let Ok(path) = media_store.path(task_id, &item.file_name)
                                        {
                                            if path.try_exists().unwrap_or(false) {
                                                ui.add(
                                                    egui::Image::new(image_uri(&path))
                                                        .max_size(egui::vec2(120.0, 80.0)),
                                                );
                                            } else {
                                                ui.label("图片缺失");
                                            }
                                        }
                                    }
                                    TaskMediaKind::Audio | TaskMediaKind::Video => {
                                        let label = match item.kind {
                                            TaskMediaKind::Audio => "▶ 音频",
                                            TaskMediaKind::Video => "▶ 视频",
                                            TaskMediaKind::Image => unreachable!(),
                                        };
                                        ui.label(label);
                                    }
                                }
                                ui.weak(&item.original_name);
                                if icons::add(ui, ButtonId::RemoveImage).clicked() {
                                    remove = Some(item.id);
                                }
                            },
                        );
                    }
                });

                if let Some(error) = &editor.error {
                    let danger = ui.visuals().error_fg_color;
                    ui.colored_label(danger, error);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("添加媒体…").clicked() {
                        add_media = true;
                    }
                    if icons::add(ui, ButtonId::Save).clicked() {
                        save = true;
                    }
                    if icons::add(ui, ButtonId::Cancel).clicked() {
                        close = true;
                    }
                });
            });

        if !open || close {
            self.task_detail = None;
            return;
        }
        if add_media
            && let Some(files) = rfd::FileDialog::new()
                .set_title("选择媒体文件")
                .add_filter(
                    "媒体",
                    &[
                        "png", "jpg", "jpeg", "webp", "bmp", "mp3", "wav", "ogg", "m4a", "flac",
                        "mp4", "webm", "mov", "mkv",
                    ],
                )
                .pick_files()
        {
            self.add_task_media(task_id, files);
        }
        if let Some(id) = remove {
            self.remove_task_media(task_id, id);
        }
        if save {
            let description = self
                .task_detail
                .as_ref()
                .map(|editor| editor.description.clone())
                .unwrap_or_default();
            self.save_task_detail(task_id, &description);
        }
    }

    /// 校验并落盘一段媒体，再写入元数据。
    fn add_task_media(&mut self, task_id: TaskId, files: Vec<PathBuf>) {
        let media_store = MediaStore::new(&self.data_dir);
        for file in files {
            let extension = file
                .extension()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase())
                .unwrap_or_default();
            let kind = if ["png", "jpg", "jpeg", "webp", "bmp"].contains(&extension.as_str()) {
                TaskMediaKind::Image
            } else if ["mp4", "webm", "mov", "mkv"].contains(&extension.as_str()) {
                TaskMediaKind::Video
            } else {
                TaskMediaKind::Audio
            };
            match media_store.store(task_id, &file, kind) {
                Ok(stored) => {
                    if let Err(error) = self.store.add_task_media(
                        task_id,
                        kind,
                        &stored.file_name,
                        &stored.original_name,
                        stored.byte_size,
                    ) {
                        let _ = media_store.remove(task_id, &stored.file_name);
                        warn!(
                            op = "add_task_media",
                            result = "error",
                            task = task_id.0,
                            error = %error,
                            "媒体元数据写入失败，已清理本地副本"
                        );
                        self.error = Some(format!("记录媒体失败：{error}"));
                    }
                }
                Err(error) => {
                    warn!(
                        op = "add_task_media",
                        result = "error",
                        task = task_id.0,
                        error = %error,
                        "媒体文件校验失败"
                    );
                    self.error = Some(format!("添加媒体失败：{error}"));
                }
            }
        }
        if let Ok(media) = self.store.task_media(task_id)
            && let Some(editor) = self.task_detail.as_mut()
        {
            editor.media = media;
            editor.error = None;
        }
    }

    /// 删除一段媒体：先删元数据，再清理本地文件。
    fn remove_task_media(&mut self, task_id: TaskId, id: TaskMediaId) {
        let media_store = MediaStore::new(&self.data_dir);
        match self.store.delete_task_media(id) {
            Ok(record) => {
                let _ = media_store.remove(task_id, &record.file_name);
            }
            Err(error) => {
                warn!(
                    op = "remove_task_media",
                    result = "error",
                    task = task_id.0,
                    error = %error,
                    "媒体元数据删除失败"
                );
                self.error = Some(format!("删除媒体失败：{error}"));
            }
        }
        if let Ok(media) = self.store.task_media(task_id)
            && let Some(editor) = self.task_detail.as_mut()
        {
            editor.media = media;
        }
    }

    /// 保存任务内容文本。
    pub(crate) fn save_task_detail(&mut self, task_id: TaskId, description: &str) {
        match self.store.set_task_description(task_id, description) {
            Ok(()) => {
                self.task_detail = None;
            }
            Err(error) => {
                warn!(
                    op = "save_task_detail",
                    result = "error",
                    task = task_id.0,
                    error = %error,
                    "任务内容保存失败"
                );
                if let Some(editor) = self.task_detail.as_mut() {
                    editor.error = Some(format!("保存失败：{error}"));
                }
            }
        }
    }
}
