//! 改频率弹窗：模式、参数与保存。

use eframe::egui;

use crate::app::{FrequencyMode, MinActionApp};
use crate::model::{Frequency, TaskId};

impl MinActionApp {
    /// 打开改频率弹窗，预填任务当前的模式与参数。
    pub(crate) fn open_frequency_dialog(&mut self, task: TaskId, frequency: Frequency) {
        let (mode, interval) = match frequency {
            Frequency::Daily => (FrequencyMode::Daily, String::new()),
            Frequency::EveryNDays(days) => (FrequencyMode::EveryNDays, days.to_string()),
            Frequency::WeeklyN(times) => (FrequencyMode::WeeklyN, times.to_string()),
        };
        self.draft_frequency_mode = mode;
        self.draft_frequency_interval = interval;
        self.frequency_target = Some(task);
    }

    pub(crate) fn ui_frequency_dialog(&mut self, ctx: &egui::Context) {
        if self.frequency_target.is_none() {
            return;
        }

        let mut save = false;
        let mut cancel = false;

        egui::Window::new("修改频率")
            .id(egui::Id::unique("dialog-frequency"))
            .collapsible(false)
            .resizable(false)
            .default_pos([320.0, 240.0])
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("频率");
                    for (mode, text) in [
                        (FrequencyMode::Daily, "每天"),
                        (FrequencyMode::EveryNDays, "每 N 天"),
                        (FrequencyMode::WeeklyN, "每周 N 次"),
                    ] {
                        if ui
                            .selectable_label(self.draft_frequency_mode == mode, text)
                            .clicked()
                            && self.draft_frequency_mode != mode
                        {
                            self.draft_frequency_mode = mode;
                            self.draft_frequency_interval = mode.default_interval().to_string();
                        }
                    }
                });
                if self.draft_frequency_mode != FrequencyMode::Daily {
                    ui.horizontal(|ui| {
                        ui.label(self.draft_frequency_mode.interval_label());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.draft_frequency_interval)
                                .desired_width(60.0),
                        );
                    });
                }
                ui.weak("改频率不影响已经生成的行动，下一次生成按新间隔判定。");
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
            self.submit_frequency();
        } else if cancel {
            self.close_frequency_dialog();
        }
    }

    /// 提交改频率；参数不合法时保留弹窗与输入。
    pub(crate) fn submit_frequency(&mut self) {
        let Some(task) = self.frequency_target else {
            return;
        };
        let interval = self
            .draft_frequency_interval
            .trim()
            .parse::<i64>()
            .unwrap_or(0);
        match Frequency::from_parts(self.draft_frequency_mode.kind(), interval) {
            Ok(frequency) => {
                if self
                    .apply(|app| app.store.update_task_frequency(task, frequency))
                    .is_some()
                {
                    self.close_frequency_dialog();
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn close_frequency_dialog(&mut self) {
        self.frequency_target = None;
        self.draft_frequency_interval.clear();
        self.draft_frequency_mode = FrequencyMode::Daily;
    }
}
