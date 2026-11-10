//! 顶栏：当日状态、每日上限、主题入口与错误提示。

use std::time::Instant;

use eframe::egui;
use tracing::{info, warn};

use crate::app::icons::{self, ButtonId};
use crate::app::{MinActionApp, theme};
use crate::model::ThemeChoice;

impl MinActionApp {
    pub(crate) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(format!("今日 · {}", self.today.format("%Y-%m-%d")));
            ui.separator();
            let pending = self
                .actions
                .iter()
                .filter(|action| action.status == crate::model::ActionStatus::Pending)
                .count();
            ui.label(format!("共 {} 条待完成", pending));
            if self.deferred > 0 {
                ui.separator();
                let warning = ui.visuals().warn_fg_color;
                ui.colored_label(warning, format!("已顺延 {} 条", self.deferred));
            }
            ui.separator();
            ui.label("每日上限");
            let mut cap = self.daily_cap;
            if ui
                .add(egui::DragValue::new(&mut cap).range(1..=20))
                .changed()
            {
                let started = Instant::now();
                match self.store.set_daily_cap(cap) {
                    Ok(()) => {
                        self.daily_cap = cap;
                        info!(
                            op = "set_daily_cap",
                            result = "ok",
                            cap,
                            elapsed_ms = crate::common::elapsed_ms(started),
                            "每日上限已更新"
                        );
                        if let Err(error) = self.reload() {
                            self.error = Some(format!("刷新失败：{error}"));
                            warn!(
                                op = "set_daily_cap",
                                result = "refresh_error",
                                cap,
                                elapsed_ms = crate::common::elapsed_ms(started),
                                error = %error,
                                "每日上限已更新，但随后的刷新失败"
                            );
                        }
                    }
                    Err(error) => {
                        self.error = Some(error.to_string());
                        warn!(
                            op = "set_daily_cap",
                            result = "error",
                            cap,
                            elapsed_ms = crate::common::elapsed_ms(started),
                            error = %error,
                            "每日上限更新失败"
                        );
                    }
                }
            }
            ui.separator();
            let mut chosen = None;
            egui::ComboBox::from_id_salt("theme-choice")
                .selected_text(theme::label(self.theme))
                .width(96.0)
                .show_ui(ui, |ui| {
                    for value in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
                        if ui
                            .selectable_label(self.theme == value, theme::label(value))
                            .clicked()
                        {
                            chosen = Some(value);
                        }
                    }
                });
            if let Some(choice) = chosen {
                self.set_theme(choice, ui.ctx());
            }
            ui.separator();
            if icons::add(ui, ButtonId::OpenStats).clicked() {
                self.open_stats();
            }
        });

        if let Some(message) = self.error.clone() {
            ui.horizontal(|ui| {
                let danger = ui.visuals().error_fg_color;
                ui.colored_label(danger, message);
                if icons::add(ui, ButtonId::Confirm).clicked() {
                    self.error = None;
                }
            });
        }
    }
}
