//! FR-016 本地日界切换。

use chrono::{Datelike, NaiveDate};
use ngy_min_action::clock::{Clock, FixedClock, SystemClock};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

#[test]
fn fixed_clock_starts_on_the_given_day() {
    let clock = FixedClock::new(date(2026, 11, 2));
    assert_eq!(clock.today(), date(2026, 11, 2));
}

#[test]
fn advancing_one_day_moves_the_boundary() {
    let clock = FixedClock::new(date(2026, 11, 2));
    clock.advance_days(1);
    assert_eq!(clock.today(), date(2026, 11, 3));
}

#[test]
fn advancing_across_month_end_keeps_calendar_arithmetic() {
    let clock = FixedClock::new(date(2026, 11, 30));
    clock.advance_days(1);
    assert_eq!(clock.today(), date(2026, 12, 1));
}

#[test]
fn setting_the_clock_back_is_supported_for_clock_rollback_cases() {
    let clock = FixedClock::new(date(2026, 11, 2));
    clock.set(date(2026, 11, 1));
    assert_eq!(clock.today(), date(2026, 11, 1));
}

#[test]
fn system_clock_reports_a_usable_date() {
    let today = SystemClock.today();
    assert!(today.year() >= 2020, "系统时钟应给出合理年份");
}
