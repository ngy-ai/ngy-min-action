//! 预计耗时编辑弹窗。

use eframe::egui;

use crate::app::MinActionApp;
use crate::model::{DomainError, MAX_ESTIMATED_MINUTES, TaskId, validate_estimated_minutes};

impl MinActionApp {
    /// 打开耗时编辑弹窗，预填当前值。
    pub(crate) fn open_estimate_dialog(&mut self, task: TaskId, current: Option<u16>) {
        self.draft_estimate = current
            .map(|minutes| minutes.to_string())
            .unwrap_or_default();
        self.estimate_target = Some(task);
    }

    pub(crate) fn ui_estimate_dialog(&mut self, ctx: &egui::Context) {
        if self.estimate_target.is_none() {
            return;
        }

        let mut save = false;
        let mut cancel = false;

        egui::Window::new("预计耗时")
            .id(egui::Id::unique("dialog-estimate"))
            .collapsible(false)
            .resizable(false)
            .default_pos([320.0, 240.0])
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("分钟");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.draft_estimate).desired_width(80.0),
                    );
                });
                ui.weak(format!(
                    "留空表示不估时，范围 1 到 {MAX_ESTIMATED_MINUTES} 分钟。"
                ));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        save = true;
                    }
                    if ui.button("取消").clicked() {
                        cancel = true;
                    }
                });
            });

        if save {
            self.submit_estimate();
        } else if cancel {
            self.close_estimate_dialog();
        }
    }

    /// 提交耗时；非法值保留弹窗与输入。
    pub(crate) fn submit_estimate(&mut self) {
        let Some(task) = self.estimate_target else {
            return;
        };
        let minutes = match Self::parse_estimate_text(&self.draft_estimate) {
            Ok(minutes) => minutes,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        if self
            .apply(|app| app.store.update_task_estimated_minutes(task, minutes))
            .is_some()
        {
            self.close_estimate_dialog();
        }
    }

    /// 解析耗时输入；空串与 `0` 都表示未估时。
    pub(crate) fn parse_estimate_text(text: &str) -> Result<Option<u16>, DomainError> {
        let text = text.trim();
        if text.is_empty() {
            return validate_estimated_minutes(None);
        }
        let minutes = text.parse::<u16>().map_err(|_| DomainError::InvalidValue {
            field: "预计耗时",
            expected: format!("1 到 {MAX_ESTIMATED_MINUTES} 分钟，或留空"),
        })?;
        validate_estimated_minutes(Some(minutes))
    }

    fn close_estimate_dialog(&mut self) {
        self.estimate_target = None;
        self.draft_estimate.clear();
    }
}
