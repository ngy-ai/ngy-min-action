//! 统计对话框（T014）：展示复习统计。
//!
//! 复用 `Store::review_stats`（零数据层改动）算出 [`ReviewStats`]，以只读面板列出
//! 窗口分项、完成率、连续达成天数与任务阶段分布。内容整理逻辑抽成 `stats_rows`，
//! 便于在无 GPU 的测试中直接验证。

use gpui::{Context, Entity, Render, Window};
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::model::ReviewStats;

/// 统计窗口天数（含今天），用于完成率统计。
pub(crate) const STATS_WINDOW_DAYS: u32 = 30;

/// 统计对话框视图。
pub struct StatsDialog {
    root: Entity<MinActionApp>,
    stats: ReviewStats,
}

impl StatsDialog {
    /// 用已算好的统计构造对话框。
    pub fn new(stats: ReviewStats, root: Entity<MinActionApp>) -> Self {
        Self { root, stats }
    }
}

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

impl Render for StatsDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.clone();
        let rows = stats_rows(&self.stats);
        div()
            .id("stats-dialog")
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .w(px(360.))
            .child(div().text_size(px(18.)).child("复习统计"))
            .children(rows.into_iter().map(|(label, value)| {
                div()
                    .flex()
                    .justify_between()
                    .child(div().child(label))
                    .child(div().child(value))
            }))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root)),
            )
    }
}

/// 「关闭」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("stats-close")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .child("关闭")
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |root, cx| {
                root.close_dialog();
                cx.notify();
            });
        })
}

#[cfg(test)]
mod tests;
