//! 重命名弹窗：目标、分类、任务共用一套输入与提交逻辑。

use eframe::egui;

use crate::app::{MinActionApp, RenameTarget};

/// 名称输入框的固定 ID，供打开弹窗时写入全选状态。
fn name_edit_id() -> egui::Id {
    egui::Id::unique("dialog-rename-name")
}

impl MinActionApp {
    /// 打开重命名弹窗并预填当前名称。
    pub(crate) fn open_rename_dialog(&mut self, target: RenameTarget, current: &str) {
        self.draft_rename_name = current.to_owned();
        self.rename_target = Some(target);
        self.rename_select_all = true;
    }

    pub(crate) fn ui_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.rename_target else {
            return;
        };

        let edit_id = name_edit_id();
        if self.rename_select_all {
            self.select_all_name(ctx, edit_id);
        }

        let mut save = false;
        let mut cancel = false;

        egui::Window::new(match target {
            RenameTarget::Goal(_) => "重命名目标",
            RenameTarget::Category(_) => "重命名分类",
            RenameTarget::Task(_) => "重命名任务",
        })
        .id(egui::Id::unique("dialog-rename"))
        .collapsible(false)
        .resizable(false)
        .default_pos([320.0, 240.0])
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("名称");
                ui.add(
                    egui::TextEdit::singleline(&mut self.draft_rename_name)
                        .id(edit_id)
                        .desired_width(240.0),
                );
            });
            ui.weak("校验失败时弹窗保留，可直接修改。");
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
            self.submit_rename();
        } else if cancel {
            self.close_rename_dialog();
        }
    }

    /// 提交当前的重命名；失败时保留弹窗与输入内容。
    pub(crate) fn submit_rename(&mut self) {
        let Some(target) = self.rename_target else {
            return;
        };
        let name = self.draft_rename_name.clone();
        let saved = match target {
            RenameTarget::Goal(goal) => self
                .apply(|app| app.store.rename_goal(goal, &name))
                .is_some(),
            RenameTarget::Category(category) => self
                .apply(|app| app.store.rename_category(category, &name))
                .is_some(),
            RenameTarget::Task(task) => self
                .apply(|app| app.store.rename_task(task, &name))
                .is_some(),
        };
        if saved {
            self.close_rename_dialog();
        }
    }

    fn close_rename_dialog(&mut self) {
        self.rename_target = None;
        self.draft_rename_name.clear();
        self.rename_select_all = false;
    }

    /// 把输入框内容整体选中，便于直接键入覆盖。
    fn select_all_name(&mut self, ctx: &egui::Context, edit_id: egui::Id) {
        let mut state =
            egui::widgets::text_edit::TextEditState::load(ctx, edit_id).unwrap_or_default();
        let end = egui::text::CCursor::new(self.draft_rename_name.chars().count());
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                end,
            )));
        state.store(ctx, edit_id);
        ctx.memory_mut(|memory| memory.request_focus(edit_id));
        self.rename_select_all = false;
    }
}
