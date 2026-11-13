//! 频率对话框解析逻辑无头验收（T014）。
//!
//! 对话框的核心是 `resolve_frequency`：把用户选择的种类与输入框文本解析成 `Frequency`，
//! 并对非法数值给出可读错误。这里直接验证该纯函数，无需 GPU。

use super::FrequencyKind;
use super::resolve_frequency;
use crate::model::Frequency;

#[test]
fn daily_needs_no_value() {
    assert_eq!(
        resolve_frequency(FrequencyKind::Daily, ""),
        Ok(Frequency::Daily)
    );
    assert_eq!(
        resolve_frequency(FrequencyKind::Daily, "随便填"),
        Ok(Frequency::Daily),
        "每天模式忽略数值输入"
    );
}

#[test]
fn every_n_days_parses_valid_range() {
    assert_eq!(
        resolve_frequency(FrequencyKind::EveryNDays, "3"),
        Ok(Frequency::EveryNDays(3))
    );
}

#[test]
fn every_n_days_rejects_out_of_range() {
    assert!(resolve_frequency(FrequencyKind::EveryNDays, "0").is_err());
    assert!(resolve_frequency(FrequencyKind::EveryNDays, "366").is_err());
    assert!(resolve_frequency(FrequencyKind::EveryNDays, "abc").is_err());
}

#[test]
fn weekly_n_parses_valid_range() {
    assert_eq!(
        resolve_frequency(FrequencyKind::WeeklyN, "2"),
        Ok(Frequency::WeeklyN(2))
    );
}

#[test]
fn weekly_n_rejects_out_of_range() {
    assert!(resolve_frequency(FrequencyKind::WeeklyN, "0").is_err());
    assert!(resolve_frequency(FrequencyKind::WeeklyN, "8").is_err());
    assert!(resolve_frequency(FrequencyKind::WeeklyN, "x").is_err());
}

#[test]
fn kind_inferred_from_current_frequency() {
    use super::FrequencyKind::*;
    assert_eq!(FrequencyKind::from_frequency(Frequency::Daily), Daily);
    assert_eq!(
        FrequencyKind::from_frequency(Frequency::EveryNDays(4)),
        EveryNDays
    );
    assert_eq!(
        FrequencyKind::from_frequency(Frequency::WeeklyN(5)),
        WeeklyN
    );
}
