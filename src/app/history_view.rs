//! 历史待办：按日期倒序的游标分页、备注查看与结算入口。
//!
//! 历史只读备注，但完成／跳过／撤销与今日共用同一套 store 方法，避免两份规则。

use eframe::egui;

use crate::app::MinActionApp;
use crate::app::describe::status_text;
use crate::app::icons::{self, ButtonId};
use crate::common::HISTORY_PAGE_SIZE;
use crate::model::{ActionId, ActionStatus};

impl MinActionApp {
    /// 首次进入历史页签时加载第一页。
    pub(crate) fn ensure_history(&mut self) {
        if self.history.loaded {
            return;
        }
        self.load_history_page(true);
    }

    /// 清空历史缓存，下一次进入时重新取第一页。
    pub(crate) fn reset_history(&mut self) {
        self.history = Default::default();
    }

    /// 加载一页历史；`first` 为真时覆盖缓存，否则追加。
    pub(crate) fn load_history_page(&mut self, first: bool) {
        let cursor = if first { None } else { self.history.cursor };
        match self
            .store
            .actions_page(self.today, cursor, HISTORY_PAGE_SIZE)
        {
            Ok(page) => {
                if first {
                    self.history.actions = page.actions;
                } else {
                    self.history.actions.extend(page.actions);
                }
                self.history.cursor = page.next_cursor;
                self.history.has_more = page.has_more;
                self.history.loaded = true;
                self.history.error = None;
            }
            Err(error) => {
                self.history.error = Some(format!("读取历史待办失败：{error}"));
                self.history.loaded = true;
            }
        }
    }

    pub(crate) fn ui_history_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("历史待办");
        ui.label("这里只显示今天之前的待办；备注与图片与当时保存的一致。");

        if let Some(error) = self.history.error.clone() {
            ui.horizontal(|ui| {
                let danger = ui.visuals().error_fg_color;
                ui.colored_label(danger, error);
                if ui.button("重试").clicked() {
                    self.reset_history();
                    self.ensure_history();
                }
            });
        }

        if self.history.actions.is_empty() {
            if self.history.loaded {
                ui.add_space(8.0);
                ui.label("还没有历史待办。");
                ui.label("从明天开始，今天之前的待办会出现在这里。");
            } else {
                ui.spinner();
            }
            return;
        }

        let rows: Vec<HistoryRow> = self
            .history
            .actions
            .iter()
            .map(|action| HistoryRow {
                id: action.id,
                date: action.on_date,
                text: self.describe_action(action),
                status: action.status,
                has_note: action.note.is_some(),
            })
            .collect();

        let mut settle: Option<(ActionId, ActionStatus)> = None;
        let mut undo: Option<ActionId> = None;
        let mut open_note: Option<ActionId> = None;

        egui::ScrollArea::vertical()
            .id_salt("history-actions")
            .max_height(340.0)
            .show(ui, |ui| {
                let mut last_date = None;
                for row in &rows {
                    if last_date != Some(row.date) {
                        ui.add_space(6.0);
                        ui.strong(row.date.format("%Y-%m-%d").to_string());
                        last_date = Some(row.date);
                    }
                    ui.horizontal(|ui| {
                        ui.label(&row.text);
                        if row.has_note {
                            ui.label("有备注");
                        }
                        if icons::add(ui, ButtonId::Note).clicked() {
                            open_note = Some(row.id);
                        }
                        if row.status == ActionStatus::Pending {
                            if icons::add(ui, ButtonId::Done).clicked() {
                                settle = Some((row.id, ActionStatus::Done));
                            }
                            if icons::add(ui, ButtonId::Skip).clicked() {
                                settle = Some((row.id, ActionStatus::Skipped));
                            }
                        } else {
                            ui.label(status_text(row.status));
                            if row.status.is_settled() && icons::add(ui, ButtonId::Undo).clicked() {
                                undo = Some(row.id);
                            }
                        }
                    });
                }
            });

        ui.separator();
        if self.history.has_more {
            if ui.button("载入更早").clicked() {
                self.load_history_page(false);
            }
        } else {
            ui.label("没有更早的待办了。");
        }

        if let Some((action, next)) = settle {
            let hint = if next == ActionStatus::Done {
                "已完成"
            } else {
                "已跳过"
            };
            self.apply(|app| {
                app.store
                    .settle_action(action, next)
                    .map(|()| hint.to_owned())
            });
            self.reset_history();
            self.ensure_history();
        }
        if let Some(action) = undo {
            self.apply(|app| {
                app.store
                    .undo_last_settlement(action)
                    .map(|()| "已撤销".to_owned())
            });
            self.reset_history();
            self.ensure_history();
        }
        if let Some(action) = open_note {
            self.open_note(action, true);
        }
    }
}

struct HistoryRow {
    id: ActionId,
    date: chrono::NaiveDate,
    text: String,
    status: ActionStatus,
    has_note: bool,
}
