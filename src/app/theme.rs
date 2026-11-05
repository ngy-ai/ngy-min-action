//! 界面视觉令牌：两套主题共用的间距、字号层级与语义配色。
//!
//! 所有颜色字面量只在本文件出现；界面其余代码读取 `ui.visuals()` 的语义字段。

use eframe::egui::{self, Color32, FontFamily, FontId, TextStyle};

use crate::model::ThemeChoice;

/// 标题字号（逻辑像素）。
const HEADING_SIZE: f32 = 20.0;

/// 正文字号（逻辑像素）。
const BODY_SIZE: f32 = 14.0;

/// 生成指定主题的样式；测试可直接读取其中的令牌。
pub(crate) fn style_for(theme: egui::Theme) -> egui::Style {
    let mut style = theme.default_style();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.indent = 18.0;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(HEADING_SIZE, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(BODY_SIZE, FontFamily::Proportional),
    );

    match theme {
        egui::Theme::Light => {
            style.visuals.weak_text_color = Some(Color32::from_rgb(0x55, 0x55, 0x55));
            style.visuals.warn_fg_color = Color32::from_rgb(0x8A, 0x4B, 0x00);
            style.visuals.error_fg_color = Color32::from_rgb(0xB0, 0x00, 0x20);
            style.visuals.hyperlink_color = Color32::from_rgb(0x06, 0x45, 0xAD);
        }
        egui::Theme::Dark => {
            style.visuals.weak_text_color = Some(Color32::from_rgb(0xC0, 0xC0, 0xC0));
            style.visuals.warn_fg_color = Color32::from_rgb(0xFF, 0xB7, 0x4D);
            style.visuals.error_fg_color = Color32::from_rgb(0xFF, 0x8A, 0x80);
            style.visuals.hyperlink_color = Color32::from_rgb(0x8A, 0xB4, 0xF8);
        }
    }
    style
}

/// 把两套样式注册到界面上下文，供主题切换时自动选择。
pub(crate) fn apply_styles(ctx: &egui::Context) {
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.set_style_of(theme, style_for(theme));
    }
}

/// 把领域层的偏好映射为界面框架的主题设置。
pub(crate) fn preference(choice: ThemeChoice) -> egui::ThemePreference {
    match choice {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Light => egui::ThemePreference::Light,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
    }
}

/// WCAG 相对亮度对比度；用于自动验证两套令牌的可读性。
pub(crate) fn contrast_ratio(foreground: Color32, background: Color32) -> f32 {
    fn luminance(color: Color32) -> f32 {
        let channels = [color.r(), color.g(), color.b()].map(|value| {
            let channel = f32::from(value) / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
    }

    let front = luminance(foreground);
    let back = luminance(background);
    (front.max(back) + 0.05) / (front.min(back) + 0.05)
}
