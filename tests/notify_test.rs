//! FR-024 ~ FR-031 每日提示判定。

use chrono::NaiveDate;
use ngy_min_action::model::ReminderState;
use ngy_min_action::notify::should_notify;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

fn state(notified: Option<NaiveDate>, viewed: Option<NaiveDate>) -> ReminderState {
    ReminderState {
        last_notified_on: notified,
        last_viewed_on: viewed,
    }
}

#[test]
fn notifies_once_on_a_fresh_day() {
    assert!(should_notify(&state(None, None), date(2026, 11, 2), 3));
}

#[test]
fn does_not_notify_without_pending_actions() {
    assert!(!should_notify(&state(None, None), date(2026, 11, 2), 0));
}

#[test]
fn does_not_notify_twice_on_the_same_day() {
    let today = date(2026, 11, 2);
    assert!(!should_notify(&state(Some(today), None), today, 3));
}

#[test]
fn does_not_notify_after_the_list_was_viewed() {
    let today = date(2026, 11, 2);
    assert!(!should_notify(&state(None, Some(today)), today, 3));
}

#[test]
fn notifies_again_on_the_next_day() {
    let yesterday = date(2026, 11, 2);
    assert!(should_notify(
        &state(Some(yesterday), Some(yesterday)),
        date(2026, 11, 3),
        2
    ));
}

#[test]
fn notify_state_survives_a_restart() {
    // 重启后记录仍在库里，因此同一本地日不会再弹一次：这里以「记录已存在」模拟重启。
    let today = date(2026, 11, 2);
    let restored = state(Some(today), None);
    assert!(!should_notify(&restored, today, 5));
}

#[test]
fn decision_is_stable_within_the_same_day() {
    let today = date(2026, 11, 2);
    let current = state(None, None);
    let first = should_notify(&current, today, 1);
    let second = should_notify(&current, today, 1);
    assert_eq!(first, second);
}
