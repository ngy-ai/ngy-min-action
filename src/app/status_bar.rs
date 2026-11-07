//! 底部状态栏：数据位置、今日待办数与版本。
//!
//! 长路径按字符中间省略，完整路径放在悬停提示里，避免挤压右侧计数。

use eframe::egui;

use crate::app::MinActionApp;
use crate::model::ActionStatus;

impl MinActionApp {
    pub(crate) fn ui_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let path = self.data_dir.display().to_string();
            ui.add(egui::Label::new(elide_middle(&path, 64)).truncate())
                .on_hover_text(path);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                let pending = self
                    .actions
                    .iter()
                    .filter(|action| action.status == ActionStatus::Pending)
                    .count();
                ui.label(format!("今日待办 {pending} 条"));
            });
        });
    }
}

/// 超过 `max_chars` 时保留首尾，中间用省略号。
pub(crate) fn elide_middle(text: &str, max_chars: usize) -> String {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= max_chars {
        return text.to_owned();
    }
    let keep = max_chars.saturating_sub(1);
    let head = keep / 2;
    let tail = keep - head;
    let mut result: String = characters[..head].iter().collect();
    result.push('…');
    result.extend(&characters[characters.len() - tail..]);
    result
}
