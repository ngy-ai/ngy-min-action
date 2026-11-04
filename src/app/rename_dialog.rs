//! 重命名弹窗：目标、分类、任务共用一套输入与提交逻辑。

use eframe::egui;

use crate::app::{MinActionApp, RenameTarget};

impl MinActionApp {
    /// 打开重命名弹窗并预填当前名称。
    pub(crate) fn open_rename_dialog(&mut self, target: RenameTarget, current: &str) {
        self.draft_rename_name = current.to_owned();
        self.rename_target = Some(target);
    }

    pub(crate) fn ui_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.rename_target else {
            return;
        };

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
                ui.text_edit_singleline(&mut self.draft_rename_name);
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
        } else if cancel {
            self.close_rename_dialog();
        }
    }

    fn close_rename_dialog(&mut self) {
        self.rename_target = None;
        self.draft_rename_name.clear();
    }
}
