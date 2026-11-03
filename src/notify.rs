//! 每日提示的判定规则。
//!
//! 判定与落库分离：本模块只回答「今天该不该弹」，把 `last_notified_on` 写回去是调用方
//! 的责任，顺序必须是「先判定、再弹窗、弹成功后落库」。

use chrono::NaiveDate;

use crate::model::ReminderState;

/// 判断当天是否应弹出提示窗口。
///
/// 三条同时成立才弹：今天还没弹过、今天还没看过清单、当天存在待办。
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use ngy_min_action::model::ReminderState;
/// use ngy_min_action::notify::should_notify;
///
/// let state = ReminderState::default();
/// let today = NaiveDate::from_ymd_opt(2026, 11, 2).expect("合法日期");
/// assert!(should_notify(&state, today, 3));
/// assert!(!should_notify(&state, today, 0));
/// ```
#[must_use]
pub fn should_notify(state: &ReminderState, today: NaiveDate, pending: usize) -> bool {
    pending > 0 && state.last_notified_on != Some(today) && state.last_viewed_on != Some(today)
}
