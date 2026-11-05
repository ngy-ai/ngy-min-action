//! 009：主题令牌与切换的 headless 验收。

use eframe::egui;

use crate::app::theme::{contrast_ratio, style_for};

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
