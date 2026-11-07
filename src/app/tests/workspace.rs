//! 菜单栏、状态栏与页签的 headless 验收。

use super::*;

use crate::app::TodoTab;
use crate::app::status_bar::elide_middle;
use crate::app::theme::preference;
use crate::model::ThemeChoice;
use eframe::egui;

fn seed_one_task(app: &mut MinActionApp) {
    let category = seed_category(app, "学习英语", "口语");
    app.store
        .create_task(category, draft("跟读十分钟"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
}

#[test]
fn workspace_shell_renders_in_both_themes() {
    let mut app = open_app("workspace-render");
    seed_one_task(&mut app);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.show_about = true;
    app.show_storage = true;
    app.storage.target = Some(std::env::temp_dir().join("ngy-min-action-workspace-target"));

    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(preference(choice));
        for tab in [TodoTab::Today, TodoTab::History] {
            app.switch_tab(tab);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                app.ui_menu_bar(ui);
                app.toolbar(ui);
                app.ui_status_bar(ui);
                app.ui_tabs(ui);
                let inner = ui.ctx().clone();
                app.ui_about_dialog(&inner);
                app.ui_storage_dialog(&inner);
            });
            output.textures_delta.clear();
        }
    }
}

#[test]
fn switching_tabs_loads_history_only_once() {
    let mut app = open_app("workspace-tabs");
    seed_one_task(&mut app);

    app.switch_tab(TodoTab::History);
    assert!(app.history.loaded, "首次进入历史应加载");
    assert_eq!(app.history.actions.len(), 0, "今天没有历史待办");

    app.switch_tab(TodoTab::Today);
    app.switch_tab(TodoTab::History);
    assert!(app.history.loaded, "再次进入不应重置");
}

#[test]
fn status_bar_elides_long_paths() {
    let short = "C:\\data";
    assert_eq!(elide_middle(short, 20), short);

    let long = "C:\\Users\\admin\\AppData\\Local\\ngy-min-action\\data";
    let elided = elide_middle(long, 20);
    assert_eq!(elided.chars().count(), 20);
    assert!(elided.contains('…'));
    assert!(elided.starts_with("C:\\Use"));
    assert!(elided.ends_with("tion\\data"));
}
