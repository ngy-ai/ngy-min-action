//! 菜单栏：文件、视图、帮助，以及只读的「关于」窗口。
//!
//! 菜单项用文字，不扩展 010 的图标映射；主题入口仍留在工具条，避免两处重复。

use eframe::egui;

use crate::app::{MinActionApp, TodoTab};

impl MinActionApp {
    pub(crate) fn ui_menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut open_storage = false;
        let mut switch_to = None;
        let mut open_about = false;

        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("文件", |ui| {
                if ui.button("设置存储位置…").clicked() {
                    open_storage = true;
                    ui.close();
                }
            });
            ui.menu_button("视图", |ui| {
                if ui.button("今日待办").clicked() {
                    switch_to = Some(TodoTab::Today);
                    ui.close();
                }
                if ui.button("历史待办").clicked() {
                    switch_to = Some(TodoTab::History);
                    ui.close();
                }
            });
            ui.menu_button("帮助", |ui| {
                if ui.button("关于").clicked() {
                    open_about = true;
                    ui.close();
                }
            });
        });

        if open_storage {
            self.show_storage = true;
            self.storage.error = None;
            self.storage.notice = None;
        }
        if let Some(tab) = switch_to {
            self.switch_tab(tab);
        }
        if open_about {
            self.show_about = true;
        }
    }

    pub(crate) fn ui_about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let mut open = true;
        let version = env!("CARGO_PKG_VERSION");
        let data_dir = self.data_dir.display().to_string();
        egui::Window::new("关于")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("最小行动 v{version}"));
                ui.separator();
                ui.label("数据位置");
                ui.add(egui::Label::new(data_dir.clone()).wrap());
                ui.separator();
                ui.weak("数据全部保存在本机，不上传、不联网。");
            });
        self.show_about = open;
    }
}
