//! 频率对话框（T014）：编辑任务的生成频率。
//!
//! 以独立 gpui 视图渲染一个居中面板，用户选择「每天 / 每 N 天 / 每周 N 次」并填 N，
//! 确认时把新频率写回 [`TaskEditor`]（`TaskEditor` 保存时统一写库），然后关闭对话框。

use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::app::task_editor::TaskEditor;
use crate::model::Frequency;

/// 频率种类可选值，对应 [`Frequency`] 的三种形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrequencyKind {
    /// 每天一次。
    Daily,
    /// 每 N 天一次。
    EveryNDays,
    /// 每周 N 次。
    WeeklyN,
}

impl FrequencyKind {
    /// 由当前频率推断初始选择的种类。
    fn from_frequency(f: Frequency) -> Self {
        match f {
            Frequency::Daily => FrequencyKind::Daily,
            Frequency::EveryNDays(_) => FrequencyKind::EveryNDays,
            Frequency::WeeklyN(_) => FrequencyKind::WeeklyN,
        }
    }

    /// 入库用的 kind 字符串，与 `Frequency::kind` 对齐。
    fn kind_str(self) -> &'static str {
        match self {
            FrequencyKind::Daily => "daily",
            FrequencyKind::EveryNDays => "every_n_days",
            FrequencyKind::WeeklyN => "weekly_n",
        }
    }

    /// 非「每天」时输入框的预填值。
    fn default_raw(f: Frequency) -> String {
        match f {
            Frequency::EveryNDays(d) => d.to_string(),
            Frequency::WeeklyN(t) => t.to_string(),
            Frequency::Daily => String::new(),
        }
    }
}

/// 频率对话框视图。
pub struct FrequencyDialog {
    editor: Entity<TaskEditor>,
    root: Entity<MinActionApp>,
    kind: FrequencyKind,
    raw: Entity<InputState>,
    error: Option<String>,
}

impl FrequencyDialog {
    /// 构造对话框，依据当前频率预选种类并填好 N。
    pub fn new(
        editor: Entity<TaskEditor>,
        root: Entity<MinActionApp>,
        current: Frequency,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let raw = cx.new(|c| {
            InputState::new(window, c).default_value(FrequencyKind::default_raw(current).as_str())
        });
        Self {
            editor,
            root,
            kind: FrequencyKind::from_frequency(current),
            raw,
            error: None,
        }
    }
}

/// 由种类与原始输入文本解析出 [`Frequency`]；非法输入返回可读的错误信息。
///
/// 抽成自由函数，便于在无 GPU 的测试中直接验证解析逻辑。
fn resolve_frequency(kind: FrequencyKind, raw: &str) -> Result<Frequency, String> {
    match kind {
        FrequencyKind::Daily => Ok(Frequency::Daily),
        FrequencyKind::EveryNDays => {
            let n = raw
                .trim()
                .parse::<u16>()
                .map_err(|_| "请输入有效的天数（1 起的整数）".to_string())?;
            Frequency::from_parts("every_n_days", i64::from(n)).map_err(|e| format!("{e}"))
        }
        FrequencyKind::WeeklyN => {
            let n = raw
                .trim()
                .parse::<u8>()
                .map_err(|_| "请输入有效的次数（1 到 7 的整数）".to_string())?;
            Frequency::from_parts("weekly_n", i64::from(n)).map_err(|e| format!("{e}"))
        }
    }
}

impl Render for FrequencyDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        let root = self.root.clone();
        let current_kind = self.kind;
        let raw_state = self.raw.clone();
        let dialog_entity = cx.entity().clone();
        let error = self.error.clone();

        div()
            .id("frequency-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(360.))
            .child(div().text_size(px(18.)).child("频率"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(kind_button(
                        dialog_entity.clone(),
                        FrequencyKind::Daily,
                        current_kind,
                    ))
                    .child(kind_button(
                        dialog_entity.clone(),
                        FrequencyKind::EveryNDays,
                        current_kind,
                    ))
                    .child(kind_button(
                        dialog_entity.clone(),
                        FrequencyKind::WeeklyN,
                        current_kind,
                    )),
            )
            .when(current_kind != FrequencyKind::Daily, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_size(px(12.)).child("数值"))
                        .child(Input::new(&raw_state)),
                )
            })
            .when_some(error, |this, e| {
                this.child(div().text_color(rgb(0xcc_66_66)).child(e))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel_button(root.clone()))
                    .child(confirm_button(dialog_entity.clone(), editor, root)),
            )
    }
}

/// 单个频率种类按钮；选中态高亮，点击切换当前种类。
fn kind_button(
    dialog: Entity<FrequencyDialog>,
    kind: FrequencyKind,
    current: FrequencyKind,
) -> impl IntoElement {
    let selected = kind == current;
    div()
        .id(format!("freq-kind-{}", kind.kind_str()))
        .px_2()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .when(selected, |this| this.bg(rgb(0x33_33_38)))
        .child(match kind {
            FrequencyKind::Daily => "每天",
            FrequencyKind::EveryNDays => "每 N 天",
            FrequencyKind::WeeklyN => "每周 N 次",
        })
        .on_click(move |_ev, _window, cx| {
            dialog.update(cx, |this, cx| {
                this.kind = kind;
                this.error = None;
                cx.notify();
            });
        })
}

/// 「取消」按钮：仅关闭对话框。
fn cancel_button(root: Entity<MinActionApp>) -> impl IntoElement {
    div()
        .id("freq-cancel")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .child("取消")
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |root, cx| {
                root.close_dialog();
                cx.notify();
            });
        })
}

/// 「确定」按钮：解析频率，写回编辑器后关闭。
fn confirm_button(
    dialog: Entity<FrequencyDialog>,
    editor: Entity<TaskEditor>,
    root: Entity<MinActionApp>,
) -> impl IntoElement {
    div()
        .id("freq-confirm")
        .px_3()
        .py_1()
        .cursor_pointer()
        .rounded_md()
        .bg(rgb(0x33_33_38))
        .child("确定")
        .on_click(move |_ev, _window, cx| {
            dialog.update(cx, |this, cx| {
                let raw = this.raw.read(cx).value().to_string();
                match resolve_frequency(this.kind, &raw) {
                    Ok(freq) => {
                        editor.update(cx, |ed, cx| {
                            ed.set_frequency(freq, cx);
                            cx.notify();
                        });
                        root.update(cx, |root, cx| {
                            root.close_dialog();
                            cx.notify();
                        });
                    }
                    Err(e) => {
                        this.error = Some(e);
                        cx.notify();
                    }
                }
            });
        })
}

#[cfg(test)]
mod tests;
