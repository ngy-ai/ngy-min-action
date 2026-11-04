//! 统计面板：完成率、连续达成天数与复习阶段分布。

use eframe::egui;

use crate::app::MinActionApp;

/// 完成率的统计窗口（含今天）。
const STATS_WINDOW_DAYS: u32 = 30;

impl MinActionApp {
    /// 打开统计面板，查询一次快照后缓存。
    pub(crate) fn open_stats(&mut self) {
        match self.store.review_stats(self.today, STATS_WINDOW_DAYS) {
            Ok(stats) => {
                self.stats = Some(stats);
                self.show_stats = true;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(crate) fn ui_stats_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_stats {
            return;
        }
        let Some(stats) = self.stats.clone() else {
            return;
        };

        let mut close = false;

        egui::Window::new("复习统计")
            .id(egui::Id::unique("dialog-stats"))
            .collapsible(false)
            .resizable(false)
            .default_pos([300.0, 110.0])
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.add_space(6.0);
                ui.heading(format!("近 {} 天", stats.window_days));
                match stats.tally.completion_percent() {
                    Some(percent) => {
                        ui.add(
                            egui::ProgressBar::new(percent as f32 / 100.0)
                                .text(format!("完成率 {percent}%")),
                        );
                        ui.label(format!(
                            "完成 {} · 跳过 {} · 过期 {}",
                            stats.tally.done, stats.tally.skipped, stats.tally.expired
                        ));
                    }
                    None => {
                        ui.weak(format!("近 {} 天还没有已结算的行动", stats.window_days));
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.heading("连续完成");
                ui.label(format!("当前连续 {} 天", stats.current_streak));
                if stats.longest_streak > 0 {
                    ui.weak(format!("历史最长 {} 天", stats.longest_streak));
                }

                ui.add_space(10.0);
                ui.separator();
                ui.heading("复习阶段");
                let total = stats.untouched_tasks + stats.learning_tasks + stats.graduated_tasks;
                if total == 0 {
                    ui.weak("还没有任务");
                } else {
                    ui.label(format!(
                        "未开始 {} · 学习中 {} · 已巩固 {}",
                        stats.untouched_tasks, stats.learning_tasks, stats.graduated_tasks
                    ));
                    ui.weak(format!("共 {total} 个任务"));
                }

                ui.add_space(12.0);
                if ui.button("关闭").clicked() {
                    close = true;
                }
            });

        if close {
            self.show_stats = false;
        }
    }
}
