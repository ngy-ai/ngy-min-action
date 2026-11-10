//! 待办备注：文字、图片附件、文件选择与拖拽。
//!
//! 图片在加入时立即复制并写库，文字在点「保存」时写库；关闭窗口未保存的文字丢弃，
//! 已加入的图片保留（它们已经是独立记录，可以逐张移除）。

use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui;
use tracing::{info, warn};

use crate::app::MinActionApp;
use crate::app::icons::{self, ButtonId};
use crate::attachment::AttachmentStore;
use crate::common::{NOTE_MAX_CHARS, elapsed_ms};
use crate::model::{ActionId, ActionImageId};

/// 把磁盘路径构造成图片加载器能识别的 `file://` URI。
///
/// 盘符路径必须写成 `file:///C:/dir/a.png`：写成 `file://C:\dir\a.png` 时，
/// 加载器会把 `C:\dir` 当作 UNC 主机名去找网络路径。UNC 路径则保留主机段，
/// 写成 `file://server/share/a.png`。
pub(crate) fn image_uri(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    match text.strip_prefix("//") {
        Some(unc) => format!("file://{unc}"),
        None => format!("file:///{}", text.trim_start_matches('/')),
    }
}

impl MinActionApp {
    /// 打开某条待办的备注；`read_only` 为真时只展示，不提供编辑入口。
    pub(crate) fn open_note(&mut self, action: ActionId, read_only: bool) {
        let started = Instant::now();
        let title = self
            .actions
            .iter()
            .chain(self.history.actions.iter())
            .find(|item| item.id == action)
            .map_or_else(
                || format!("行动#{}", action.0),
                |item| self.describe_action(item),
            );
        let text = match self.store.action_note(action) {
            Ok(note) => note.unwrap_or_default(),
            Err(error) => {
                self.error = Some(format!("读取备注失败：{error}"));
                warn!(
                    op = "open_note",
                    result = "error",
                    action = action.0,
                    error = %error,
                    "备注读取失败"
                );
                return;
            }
        };
        let images = match self.store.action_images(action) {
            Ok(images) => images,
            Err(error) => {
                self.error = Some(format!("读取图片附件失败：{error}"));
                warn!(
                    op = "open_note",
                    result = "error",
                    action = action.0,
                    error = %error,
                    "图片附件读取失败"
                );
                return;
            }
        };
        self.note = Some(crate::app::NoteEditor {
            action,
            title,
            text,
            images,
            read_only,
            error: None,
        });
        info!(
            op = "open_note",
            result = "ok",
            action = action.0,
            read_only,
            elapsed_ms = elapsed_ms(started),
            "备注窗口已打开"
        );
    }

    pub(crate) fn ui_note_dialog(&mut self, ctx: &egui::Context) {
        let Some(editor) = self.note.as_mut() else {
            return;
        };
        let action = editor.action;
        let read_only = editor.read_only;
        let attachments = AttachmentStore::new(&self.data_dir);
        let mut open = true;
        let mut save = false;
        let mut close = false;
        let mut choose_images = false;
        let mut remove: Option<ActionImageId> = None;

        egui::Window::new("待办备注")
            .open(&mut open)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(&editor.title);
                ui.separator();

                if read_only {
                    if editor.text.trim().is_empty() {
                        ui.weak("没有文字备注");
                    } else {
                        ui.label(&editor.text);
                    }
                } else {
                    ui.add(
                        egui::TextEdit::multiline(&mut editor.text)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .char_limit(NOTE_MAX_CHARS),
                    );
                    ui.weak(format!(
                        "{}/{} 个字符",
                        editor.text.chars().count(),
                        NOTE_MAX_CHARS
                    ));
                }

                if let Some(error) = &editor.error {
                    let danger = ui.visuals().error_fg_color;
                    ui.colored_label(danger, error);
                }

                ui.separator();
                ui.label(format!("图片（{}）", editor.images.len()));
                if editor.images.is_empty() {
                    ui.weak("还没有图片附件。可点「添加图片」或把图片拖进窗口。");
                }
                ui.horizontal_wrapped(|ui| {
                    for image in &editor.images {
                        let path = attachments.path(action, &image.file_name).ok();
                        let exists = path.as_ref().is_some_and(|path| path.exists());
                        ui.allocate_ui_with_layout(
                            egui::vec2(128.0, 120.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                if let Some(path) = &path
                                    && exists
                                {
                                    let uri = image_uri(path);
                                    ui.add(egui::Image::new(uri).max_size(egui::vec2(96.0, 80.0)));
                                } else {
                                    ui.label("图片缺失");
                                }
                                ui.weak(&image.original_name);
                                if !read_only && icons::add(ui, ButtonId::RemoveImage).clicked() {
                                    remove = Some(image.id);
                                }
                            },
                        );
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if read_only {
                        if icons::add(ui, ButtonId::Close).clicked() {
                            close = true;
                        }
                    } else {
                        if ui.button("添加图片…").clicked() {
                            choose_images = true;
                        }
                        if icons::add(ui, ButtonId::Save).clicked() {
                            save = true;
                        }
                        if icons::add(ui, ButtonId::Cancel).clicked() {
                            close = true;
                        }
                    }
                });
            });

        if !open || close {
            self.note = None;
            return;
        }
        if choose_images
            && let Some(files) = rfd::FileDialog::new()
                .set_title("选择图片")
                .add_filter("图片", &["png", "jpg", "jpeg", "webp", "bmp"])
                .pick_files()
        {
            self.add_note_images(action, files);
        }
        if !read_only {
            let dropped: Vec<PathBuf> = ctx.input(|input| {
                input
                    .raw
                    .dropped_files
                    .iter()
                    .map(|file| file.path().to_path_buf())
                    .collect()
            });
            if !dropped.is_empty() {
                self.add_note_images(action, dropped);
            }
        }
        if let Some(image) = remove {
            self.remove_note_image(action, image);
        }
        if save {
            self.save_note(action);
        }
    }

    pub(crate) fn add_note_images(&mut self, action: ActionId, paths: Vec<PathBuf>) {
        let attachments = AttachmentStore::new(&self.data_dir);
        for path in paths {
            match attachments.store(action, &path) {
                Ok(stored) => {
                    match self.store.add_action_image(
                        action,
                        &stored.file_name,
                        &stored.original_name,
                        stored.byte_size,
                    ) {
                        Ok(image) => {
                            if let Some(editor) = self.note.as_mut() {
                                editor.images.push(image);
                                editor.error = None;
                            }
                            info!(
                                op = "add_note_image",
                                result = "ok",
                                action = action.0,
                                bytes = stored.byte_size,
                                "图片附件已加入"
                            );
                        }
                        Err(error) => {
                            let _ = attachments.remove(action, &stored.file_name);
                            self.set_note_error(format!("保存图片记录失败：{error}"));
                            warn!(
                                op = "add_note_image",
                                result = "error",
                                action = action.0,
                                error = %error,
                                "图片记录写入失败，已删除副本"
                            );
                        }
                    }
                }
                Err(error) => {
                    self.set_note_error(error.to_string());
                    warn!(
                        op = "add_note_image",
                        result = "rejected",
                        file = %path.display(),
                        error = %error,
                        "图片未通过校验"
                    );
                }
            }
        }
    }

    pub(crate) fn remove_note_image(&mut self, action: ActionId, image: ActionImageId) {
        match self.store.delete_action_image(image) {
            Ok(removed) => {
                let attachments = AttachmentStore::new(&self.data_dir);
                if let Err(error) = attachments.remove(removed.action_id, &removed.file_name) {
                    warn!(
                        op = "remove_note_image",
                        result = "file_error",
                        action = action.0,
                        error = %error,
                        "图片记录已删除，但副本清理失败"
                    );
                }
                if let Some(editor) = self.note.as_mut() {
                    editor.images.retain(|item| item.id != removed.id);
                    editor.error = None;
                }
                info!(
                    op = "remove_note_image",
                    result = "ok",
                    action = action.0,
                    "图片附件已移除"
                );
            }
            Err(error) => {
                self.set_note_error(format!("移除图片失败：{error}"));
                warn!(
                    op = "remove_note_image",
                    result = "error",
                    action = action.0,
                    error = %error,
                    "图片删除失败"
                );
            }
        }
    }

    pub(crate) fn save_note(&mut self, action: ActionId) {
        let Some(editor) = self.note.as_ref() else {
            return;
        };
        let text = editor.text.clone();
        match self.store.set_action_note(action, &text) {
            Ok(()) => {
                self.note = None;
                if let Err(error) = self.reload() {
                    self.error = Some(format!("备注已保存，但刷新失败：{error}"));
                }
                if self.history.loaded {
                    self.reset_history();
                    self.ensure_history();
                }
                info!(
                    op = "save_note",
                    result = "ok",
                    action = action.0,
                    "备注已保存"
                );
            }
            Err(error) => {
                self.set_note_error(error.to_string());
                warn!(
                    op = "save_note",
                    result = "error",
                    action = action.0,
                    error = %error,
                    "备注保存失败"
                );
            }
        }
    }

    fn set_note_error(&mut self, message: String) {
        if let Some(editor) = self.note.as_mut() {
            editor.error = Some(message);
        }
    }
}
