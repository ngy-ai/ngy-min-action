//! 随主题（浅/深）自适应的取色板，替代原先硬编码的灰/深色值。
//!
//! 色值取自 `gpui_kit::component::theme::Theme`（`Theme::global(cx)` 返回的
//! `Theme` 会 deref 到 `ThemeColor`），在浅色与深色下均正确。

use gpui_kit::component::theme::{Theme, ThemeColor};
use gpui_kit::gpui::{App, Hsla};

/// 一处取色，深浅自适应。所有曾硬编码 `rgb(0x…)` 的 UI 改用本结构字段。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Palette {
    /// 顶栏 / 工具栏背景（对应 `title_bar`）。
    pub surface: Hsla,
    /// 内嵌面板背景（如今日 / 历史内页、`muted`）。
    pub panel: Hsla,
    /// 对话框 / 卡片背景（对应 `popover`）。
    pub raised: Hsla,
    /// 列表 / 树选中行背景（对应 `list_active`）。
    pub selected: Hsla,
    /// 激活的分段控件 / 页签背景（对应 `tab_active`）。
    pub active_pill: Hsla,
    /// 非激活分段控件背景（对应 `muted`）。
    pub inactive_pill: Hsla,
    /// 弱化文字（对应 `muted_foreground`）。
    pub text_muted: Hsla,
    /// 次级文字（对应 `secondary_foreground`）。
    pub text_muted2: Hsla,
    /// 常规文字（对应 `foreground`）。
    pub text: Hsla,
    /// 危险操作背景（对应 `danger`）。
    pub danger: Hsla,
    /// 危险 / 错误文字（对应 `danger_foreground`）。
    pub danger_fg: Hsla,
}

impl Palette {
    /// 从当前全局主题取色板。
    pub(crate) fn from_cx(cx: &App) -> Self {
        let t = Theme::global(cx);
        Self {
            surface: t.title_bar,
            panel: t.muted,
            raised: t.popover,
            selected: t.list_active,
            active_pill: t.tab_active,
            inactive_pill: t.muted,
            text_muted: t.muted_foreground,
            text_muted2: t.secondary_foreground,
            text: t.foreground,
            danger: t.danger,
            danger_fg: t.danger_foreground,
        }
    }
}

impl Default for Palette {
    /// 初次构造（尚无 `cx`）时的回退值，取深色主题色，渲染前会被覆盖。
    fn default() -> Self {
        let t = ThemeColor::dark();
        Self {
            surface: t.title_bar,
            panel: t.muted,
            raised: t.popover,
            selected: t.list_active,
            active_pill: t.tab_active,
            inactive_pill: t.muted,
            text_muted: t.muted_foreground,
            text_muted2: t.secondary_foreground,
            text: t.foreground,
            danger: t.danger,
            danger_fg: t.danger_foreground,
        }
    }
}
