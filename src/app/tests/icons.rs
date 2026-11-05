//! 010：图标按钮的 headless 验收。

use eframe::egui;

use super::*;
use crate::app::icons::{self, ButtonId, IconButton, Semantic};
use crate::app::theme;
use crate::model::ThemeChoice;

#[test]
fn every_button_has_a_nonempty_hover_tip() {
    for id in ButtonId::ALL {
        assert!(!icons::spec(*id).tip.is_empty(), "{id:?} 缺少悬停提示");
    }
}

#[test]
fn danger_semantics_are_limited_to_destructive_actions() {
    let danger: Vec<_> = ButtonId::ALL
        .iter()
        .copied()
        .filter(|id| icons::spec(*id).semantic == Semantic::Danger)
        .collect();
    assert_eq!(danger.len(), 5, "危险语义应恰好覆盖五个破坏性操作");
    for id in danger {
        let tip = icons::spec(id).tip;
        assert!(
            tip.contains("删除") || tip.contains("重置"),
            "{id:?} 的提示 {tip} 不像破坏性操作"
        );
    }
}

#[test]
fn main_views_with_icons_render_in_both_themes() {
    let mut app = open_app("icons-views");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(theme::preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.status_bar(ui);
            app.ui_main_view(ui);
            app.ui_today_view(ui);
        });
        output.textures_delta.clear();
    }
}

#[test]
fn every_icon_renders_in_both_themes_with_a_usable_hit_area() {
    let ctx = egui::Context::default();
    theme::apply_styles(&ctx);
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(theme::preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for id in ButtonId::ALL {
                let response = ui.add(IconButton::new(*id));
                assert!(
                    response.rect.width() >= 34.0 && response.rect.height() >= 26.0,
                    "{id:?} 命中区域过小"
                );
            }
        });
        output.textures_delta.clear();
    }
}
