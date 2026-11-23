//! 统计页（T014）：展示复习统计。
//!
//! 与任务 / 今日待办 / 历史待办同级，作为中央区第 4 个页签，不再弹窗。数据取自
//! `MinActionApp::review_stats`（切到本页签或 `reload()` 时由 `Store::review_stats` 刷新）。
//! 内容整理逻辑抽成 `stats_rows`，便于在无 GPU 的测试中直接验证。

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::ReviewStats;
use crate::palette::Palette;

use super::style::{GAP_SM, SurfaceExt, empty_state, info_row};

/// 统计窗口天数（含今天），用于完成率统计。
pub(crate) const STATS_WINDOW_DAYS: u32 = 30;

/// 把统计整理成「标签-值」行，供渲染与无头测试复用。
pub(crate) fn stats_rows(stats: &ReviewStats) -> Vec<(&'static str, String)> {
    let mut rows = Vec::new();
    rows.push(("统计窗口", format!("近 {} 天", stats.window_days)));
    rows.push(("完成", stats.tally.done.to_string()));
    rows.push(("跳过", stats.tally.skipped.to_string()));
    rows.push(("过期", stats.tally.expired.to_string()));
    let pct = stats
        .tally
        .completion_percent()
        .map(|p| format!("{p}%"))
        .unwrap_or_else(|| "—".to_string());
    rows.push(("完成率", pct));
    rows.push(("当前连续达成", format!("{} 天", stats.current_streak)));
    rows.push(("历史最长连续", format!("{} 天", stats.longest_streak)));
    rows.push(("未开始任务", stats.untouched_tasks.to_string()));
    rows.push(("复习中任务", stats.learning_tasks.to_string()));
    rows.push(("已巩固任务", stats.graduated_tasks.to_string()));
    rows
}

/// 渲染统计页：一块只读面板列出窗口分项、连续达成与任务进度。
pub(crate) fn render(app: &MinActionApp) -> impl IntoElement {
    let palette = app.palette;
    div()
        .id("stats")
        .flex()
        .flex_col()
        .gap(GAP_SM)
        .when_some(app.review_stats.as_ref(), |this, stats| {
            this.child(panel(stats_rows(stats), &palette))
        })
        .when(app.review_stats.is_none(), |this| {
            this.child(empty_state(
                "暂时读不到统计数据",
                "数据读取失败，切换页签或重启应用后再试一次。",
                &palette,
            ))
        })
}

/// 统计面板：整块卡片，逐行「标签 — 值」。
fn panel(rows: Vec<(&'static str, String)>, palette: &Palette) -> impl IntoElement {
    div()
        .id("stats-panel")
        .flex()
        .flex_col()
        .gap_1()
        .max_w(px(420.))
        .p_4()
        .surface_card(palette)
        .children(
            rows.into_iter()
                .map(|(label, value)| info_row(label, value, palette)),
        )
}
