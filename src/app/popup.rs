//! 置顶提示窗口。
//!
//! 独立视口，层级置顶、不主动夺取键盘焦点；同一天只允许存在一个实例，
//! 关闭即视为当天已提示（由调用方写回 `last_viewed_on`）。

use std::sync::Arc;
use std::sync::Mutex;

use eframe::egui;

use crate::app::icons::{self, ButtonId};
use crate::app::{MinActionApp, PopupState};

impl MinActionApp {
    pub(crate) fn show_popup(&mut self, ctx: &egui::Context) {
        let pending = {
            let guard = self
                .popup
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.pending
        };
        let Some(pending) = pending else {
            return;
        };

        let shared = Arc::clone(&self.popup);
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("reminder-popup"),
            egui::ViewportBuilder::default()
                .with_title("今天的行动")
                .with_inner_size([360.0, 150.0])
                .with_resizable(false)
                .with_always_on_top(),
            move |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(format!("今天有 {pending} 条最小行动待完成"));
                    ui.add_space(12.0);
                    if icons::add(ui, ButtonId::ViewToday).clicked() {
                        dismiss(&shared);
                    }
                });
                if ctx.input(|input| input.viewport().close_requested()) {
                    dismiss(&shared);
                }
            },
        );
    }
}

fn dismiss(shared: &Mutex<PopupState>) {
    let mut guard = shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.dismissed = true;
}
