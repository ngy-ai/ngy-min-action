//! 界面视觉令牌与可复用的样式组合。
//!
//! 颜色来自 [`Palette`]（随主题切换），字号 / 间距 / 圆角集中在此处定义，
//! 各视图只组合这些令牌，避免再散落 `px(11.)`、`rgb(0x…)` 之类的字面量。

use gpui_kit::gpui::{Hsla, Pixels, hsla, px};
use gpui_kit::{Div, FontWeight, InteractiveElement, ParentElement, Styled, div};

use crate::palette::Palette;

/// 辅助文字、角标。
pub(crate) const TEXT_XS: Pixels = px(11.);
/// 表单标签、次级说明。
pub(crate) const TEXT_SM: Pixels = px(12.);
/// 正文强调。
pub(crate) const TEXT_MD: Pixels = px(13.);
/// 页面标题。
pub(crate) const TEXT_LG: Pixels = px(16.);
/// 对话框标题。
pub(crate) const TEXT_XL: Pixels = px(18.);

/// 紧凑间距。
pub(crate) const GAP_XS: Pixels = px(4.);
/// 常规间距。
pub(crate) const GAP_SM: Pixels = px(8.);
/// 宽松间距。
pub(crate) const GAP_MD: Pixels = px(12.);
/// 区块间距。
pub(crate) const GAP_LG: Pixels = px(16.);

/// 模态遮罩：用主题无关的半透明黑，单独定义以便 UI 代码不出现颜色字面量。
pub(crate) const SCRIM: Hsla = hsla(0., 0., 0., 0.55);

/// 操作按钮的语义色调。
#[derive(Debug, Clone, Copy)]
pub(crate) enum Tone {
    /// 主要 / 确认操作。
    Primary,
    /// 次要 / 中性操作。
    Neutral,
    /// 强调操作。
    Accent,
    /// 危险操作（删除、重置等）。
    Danger,
}

/// 常用样式组合；对同时支持布局与交互样式的元素生效。
pub(crate) trait SurfaceExt {
    /// 对话框 / 浮层卡片。
    fn surface_card(self, palette: &Palette) -> Self;
    /// 列表行卡片。
    fn list_card(self, palette: &Palette) -> Self;
    /// 工具栏 / 页签按钮，激活态使用强调色。
    fn soft_button(self, palette: &Palette, active: bool) -> Self;
    /// 计数、状态等小徽标。
    fn badge(self, palette: &Palette) -> Self;
    /// 区块小标题。
    fn section_title(self, palette: &Palette) -> Self;
    /// 带语义色调的操作按钮。
    fn tone_button(self, palette: &Palette, tone: Tone) -> Self;
}

impl<T: Styled + InteractiveElement + Sized> SurfaceExt for T {
    fn surface_card(self, palette: &Palette) -> Self {
        self.rounded_lg()
            .border_1()
            .border_color(palette.border)
            .bg(palette.raised)
            .shadow_sm()
    }

    fn list_card(self, palette: &Palette) -> Self {
        let border = palette.border;
        let hover_border = palette.accent;
        self.rounded_md()
            .border_1()
            .border_color(border)
            .bg(palette.panel)
            .hover(move |this| this.border_color(hover_border))
    }

    fn soft_button(self, palette: &Palette, active: bool) -> Self {
        let base = self
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_size(TEXT_SM);
        if active {
            base.bg(palette.active_pill)
                .text_color(palette.accent_fg)
                .font_weight(FontWeight::SEMIBOLD)
        } else {
            let hover_bg = palette.inactive_pill;
            let hover_fg = palette.text;
            base.text_color(palette.text_muted2)
                .hover(move |this| this.bg(hover_bg).text_color(hover_fg))
        }
    }

    fn badge(self, palette: &Palette) -> Self {
        self.px_2()
            .py_0()
            .rounded_full()
            .bg(palette.inactive_pill)
            .text_size(TEXT_XS)
            .text_color(palette.text_muted2)
    }

    fn section_title(self, palette: &Palette) -> Self {
        self.text_size(TEXT_SM)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(palette.text_muted2)
    }

    fn tone_button(self, palette: &Palette, tone: Tone) -> Self {
        let (bg, fg, hover_bg, hover_fg) = match tone {
            Tone::Primary => (
                palette.success,
                palette.success_fg,
                palette.success_hover,
                palette.success_fg,
            ),
            Tone::Neutral => (
                palette.inactive_pill,
                palette.text_muted2,
                palette.active_pill,
                palette.text,
            ),
            Tone::Accent => (
                palette.active_pill,
                palette.accent_fg,
                palette.accent,
                palette.accent_fg,
            ),
            Tone::Danger => (
                palette.danger,
                palette.danger_fg,
                palette.danger,
                palette.danger_fg,
            ),
        };
        self.px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_size(TEXT_XS)
            .text_color(fg)
            .bg(bg)
            .hover(move |this| this.bg(hover_bg).text_color(hover_fg))
    }
}

/// 页面标题 + 一句说明，作为各页签内容区的统一开头。
pub(crate) fn page_heading(title: &'static str, subtitle: &'static str, palette: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(GAP_XS)
        .mb(GAP_MD)
        .child(
            div()
                .text_size(TEXT_LG)
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_size(TEXT_SM)
                .text_color(palette.text_muted)
                .child(subtitle),
        )
}

/// 空列表占位：给出明确的「现在可以做什么」，而不是留白。
pub(crate) fn empty_state(title: &'static str, hint: &'static str, palette: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(GAP_XS)
        .py(GAP_LG * 3)
        .child(
            div()
                .text_size(TEXT_MD)
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.text_muted2)
                .child(title),
        )
        .child(
            div()
                .text_size(TEXT_SM)
                .text_color(palette.text_muted)
                .child(hint),
        )
}

/// 对话框标题：比页面标题更醒目，与卡片内边距留出呼吸感。
pub(crate) fn dialog_title(title: &'static str) -> Div {
    div()
        .text_size(TEXT_XL)
        .font_weight(FontWeight::BOLD)
        .child(title)
}

/// 表单小标题 / 说明：统一用次级文字，不再散落 `px(12.)`。
pub(crate) fn field_label(label: &'static str, palette: &Palette) -> Div {
    div()
        .text_size(TEXT_SM)
        .text_color(palette.text_muted2)
        .child(label)
}

/// 统计 / 存储类对话框的「标签 — 值」行。
pub(crate) fn info_row(label: &'static str, value: String, palette: &Palette) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(GAP_SM)
        .py_1()
        .child(div().text_color(palette.text_muted2).child(label))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.text)
                .child(value),
        )
}
