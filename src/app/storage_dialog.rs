//! 存储位置设置：目录选择、立即迁移与结果反馈。

use std::time::Instant;

use eframe::egui;
use tracing::{info, warn};

use crate::app::MinActionApp;
use crate::common::elapsed_ms;

impl MinActionApp {
    pub(crate) fn ui_storage_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_storage {
            return;
        }
        let mut open = true;
        let mut close = false;
        let mut choose = false;
        let mut migrate = false;

        let current = self.data_dir.display().to_string();
        let target = self
            .storage
            .target
            .as_ref()
            .map_or_else(|| "尚未选择".to_owned(), |path| path.display().to_string());
        let error = self.storage.error.clone();
        let notice = self.storage.notice.clone();

        egui::Window::new("设置存储位置")
            .open(&mut open)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.label("当前数据位置");
                ui.add(egui::Label::new(current.clone()).wrap());
                ui.separator();
                ui.label("新位置");
                ui.add(egui::Label::new(target.clone()).wrap());
                ui.horizontal(|ui| {
                    if ui.button("选择目录…").clicked() {
                        choose = true;
                    }
                });
                ui.weak(
                    "迁移会复制数据库与全部图片附件，校验通过后切换，并清理原位置；失败时保留原位置。",
                );
                if let Some(error) = &error {
                    let danger = ui.visuals().error_fg_color;
                    ui.colored_label(danger, error);
                }
                if let Some(notice) = &notice {
                    ui.label(notice);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let enabled = self.storage.target.is_some();
                    if ui
                        .add_enabled(enabled, egui::Button::new("迁移并切换"))
                        .clicked()
                    {
                        migrate = true;
                    }
                    if ui.button("关闭").clicked() {
                        close = true;
                    }
                });
            });

        if choose
            && let Some(directory) = rfd::FileDialog::new()
                .set_title("选择数据目录")
                .pick_folder()
        {
            self.storage.target = Some(directory);
            self.storage.error = None;
            self.storage.notice = None;
        }
        if migrate {
            self.relocate_storage();
        }
        self.show_storage = open && !close;
    }

    pub(crate) fn relocate_storage(&mut self) {
        let Some(target) = self.storage.target.clone() else {
            self.storage.error = Some("请先选择一个新目录".to_owned());
            return;
        };
        let started = Instant::now();
        match crate::storage::relocate(&self.store, &target, &self.bootstrap) {
            Ok(outcome) => {
                let old_dir = self.data_dir.clone();
                let old_store = std::mem::replace(&mut self.store, outcome.store);
                self.data_dir = outcome.data_dir.clone();
                drop(old_store);
                self.note = None;
                self.reset_history();

                let mut message = format!("已迁移到 {}", self.data_dir.display());
                match crate::storage::cleanup_old_data(&old_dir) {
                    Ok(()) => {}
                    Err(error) => {
                        warn!(
                            op = "storage_cleanup",
                            result = "error",
                            dir = %old_dir.display(),
                            error = %error,
                            "旧位置清理失败，残留文件可手动删除"
                        );
                        message.push_str(&format!("；旧位置清理失败：{error}"));
                    }
                }
                self.storage.error = None;
                self.storage.notice = Some(message);
                if let Err(error) = self.reload() {
                    self.error = Some(format!("位置已切换，但刷新失败：{error}"));
                }
                info!(
                    op = "storage_relocate",
                    result = "switched",
                    from = %old_dir.display(),
                    to = %self.data_dir.display(),
                    elapsed_ms = elapsed_ms(started),
                    "数据位置已切换"
                );
            }
            Err(error) => {
                self.storage.error = Some(error.to_string());
                self.storage.notice = None;
                warn!(
                    op = "storage_relocate",
                    result = "error",
                    target = %target.display(),
                    error = %error,
                    elapsed_ms = elapsed_ms(started),
                    "数据迁移未完成，继续使用原位置"
                );
            }
        }
    }
}
