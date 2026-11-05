//! 009：主题令牌与切换的 headless 验收。

use eframe::egui;

use super::*;
use crate::app::theme::{contrast_ratio, preference, style_for};
use crate::app::{PendingDelete, RenameTarget};
use crate::model::ThemeChoice;

#[test]
fn both_themes_meet_contrast_budgets() {
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let visuals = style_for(theme).visuals;
        let background = visuals.panel_fill;
        let weak = visuals.weak_text_color.unwrap_or(visuals.text_color());
        assert!(
            contrast_ratio(visuals.text_color(), background) >= 4.5,
            "{theme:?} 正文对比度不足"
        );
        assert!(
            contrast_ratio(weak, background) >= 4.5,
            "{theme:?} 弱化文字对比度不足"
        );
        assert!(
            contrast_ratio(visuals.warn_fg_color, background) >= 4.5,
            "{theme:?} 警告色对比度不足"
        );
        assert!(
            contrast_ratio(visuals.error_fg_color, background) >= 4.5,
            "{theme:?} 危险色对比度不足"
        );
    }
}

#[test]
fn applying_styles_registers_both_themes() {
    let ctx = egui::Context::default();
    crate::app::theme::apply_styles(&ctx);
    ctx.set_theme(egui::ThemePreference::Light);
    assert_eq!(ctx.theme(), egui::Theme::Light);
    ctx.set_theme(egui::ThemePreference::Dark);
    assert_eq!(ctx.theme(), egui::Theme::Dark);
}

#[test]
fn switching_theme_updates_the_context_and_the_store() {
    let mut app = open_app("theme-switch");
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);

    app.set_theme(ThemeChoice::Light, &ctx);
    assert_eq!(app.theme, ThemeChoice::Light);
    assert_eq!(ctx.theme(), egui::Theme::Light);
    assert_eq!(
        app.store.theme_choice().expect("读取偏好"),
        ThemeChoice::Light
    );

    app.set_theme(ThemeChoice::Dark, &ctx);
    assert_eq!(app.theme, ThemeChoice::Dark);
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    assert_eq!(
        app.store.theme_choice().expect("读取偏好"),
        ThemeChoice::Dark
    );
}

#[test]
fn following_system_tracks_the_injected_system_theme() {
    let ctx = egui::Context::default();
    crate::app::theme::apply_styles(&ctx);
    ctx.set_theme(egui::ThemePreference::System);
    for system in [egui::Theme::Light, egui::Theme::Dark] {
        let input = egui::RawInput {
            system_theme: Some(system),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |_ui| {});
        output.textures_delta.clear();
        assert_eq!(ctx.theme(), system, "跟随系统应反映注入的系统主题");
    }
}

#[test]
fn switching_theme_does_not_change_business_snapshots() {
    let mut app = open_app("theme-data");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let actions = app.actions.clone();
    let tasks = app.tasks.clone();
    let stats = app.stats.clone();

    let ctx = egui::Context::default();
    app.set_theme(ThemeChoice::Dark, &ctx);

    assert_eq!(app.actions, actions);
    assert_eq!(app.tasks, tasks);
    assert_eq!(app.stats, stats);
}

#[test]
fn status_bar_and_stats_dialog_render_in_both_themes() {
    let mut app = open_app("theme-dialogs");
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.open_stats();

    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.status_bar(ui);
            let inner = ui.ctx().clone();
            app.ui_stats_dialog(&inner);
        });
        output.textures_delta.clear();
    }
}

#[test]
fn every_dialog_renders_in_both_themes() {
    let mut app = open_app("theme-dialogs-all");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let frequency = app.tasks[0].frequency;

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.show_new_goal = true;
    app.show_new_category = true;
    app.open_rename_dialog(RenameTarget::Task(task), "连读");
    app.request_delete(PendingDelete::Task(task));
    app.open_frequency_dialog(task, frequency);
    app.open_estimate_dialog(task, None);
    app.request_reset(task);
    app.open_stats();

    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.status_bar(ui);
            let inner = ui.ctx().clone();
            app.ui_new_goal_dialog(&inner);
            app.ui_new_category_dialog(&inner);
            app.ui_rename_dialog(&inner);
            app.ui_frequency_dialog(&inner);
            app.ui_estimate_dialog(&inner);
            app.ui_reset_dialog(&inner);
            app.ui_stats_dialog(&inner);
            app.ui_delete_dialog(&inner);
        });
        output.textures_delta.clear();
    }
}
