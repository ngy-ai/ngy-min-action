//! 结构树目标／分类行的右键菜单。

use eframe::egui;

/// 菜单可触发的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructureAction {
    Rename,
    Delete,
}

/// 菜单项：标签与动作；集中定义便于测试与逐项核对。
pub(crate) const MENU_ITEMS: &[(&str, StructureAction)] = &[
    ("改名", StructureAction::Rename),
    ("删除", StructureAction::Delete),
];

/// 绘制菜单并返回本帧被点击的动作；点击后关闭菜单。
pub(crate) fn show(ui: &mut egui::Ui) -> Option<StructureAction> {
    let mut clicked = None;
    for (label, action) in MENU_ITEMS {
        let response = if *action == StructureAction::Delete {
            let color = ui.visuals().error_fg_color;
            ui.add(egui::Button::new(egui::RichText::new(*label).color(color)))
        } else {
            ui.button(*label)
        };
        if response.clicked() {
            clicked = Some(*action);
            ui.close();
        }
    }
    clicked
}
