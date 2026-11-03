//! FR-018 ~ FR-023 遗忘曲线推进。

use chrono::NaiveDate;
use ngy_min_action::common::REVIEW_INTERVALS_DAYS;
use ngy_min_action::model::{ReviewProgress, TaskId};
use ngy_min_action::schedule::{ScheduleError, advance_review};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

fn progress(level: u8) -> ReviewProgress {
    ReviewProgress {
        task_id: TaskId(1),
        level,
        next_review_on: None,
        graduated: false,
    }
}

#[test]
fn first_completion_enters_level_one_and_schedules_next_day() {
    let result = advance_review(&progress(0), date(2026, 11, 2), &REVIEW_INTERVALS_DAYS)
        .expect("第 0 级可以推进");
    assert_eq!(result.level, 1);
    assert_eq!(result.next_review_on, Some(date(2026, 11, 3)));
    assert!(!result.graduated);
}

#[test]
fn each_level_uses_its_own_interval() {
    let completed = date(2026, 11, 2);
    for (level, expected_offset) in [(1_u8, 2_i64), (2, 4), (3, 7), (4, 15), (5, 30)] {
        let result = advance_review(&progress(level), completed, &REVIEW_INTERVALS_DAYS)
            .expect("合法级别可以推进");
        assert_eq!(result.level, level + 1);
        assert_eq!(
            result.next_review_on,
            Some(completed + chrono::Duration::days(expected_offset)),
            "第 {level} 级复习完成后应间隔 {expected_offset} 天"
        );
    }
}

#[test]
fn finishing_the_sequence_graduates_the_task() {
    let last_level = u8::try_from(REVIEW_INTERVALS_DAYS.len()).expect("序列长度足够小");
    let result = advance_review(
        &progress(last_level),
        date(2026, 11, 2),
        &REVIEW_INTERVALS_DAYS,
    )
    .expect("最后一级可以推进");
    assert!(result.graduated, "走完序列应标记为已巩固");
    assert_eq!(result.next_review_on, None, "已巩固的任务不再安排复习");
}

#[test]
fn skipping_does_not_touch_progress() {
    let before = progress(2);
    let after = advance_review(&before, date(2026, 11, 2), &REVIEW_INTERVALS_DAYS)
        .expect("合法级别可以推进");
    assert_eq!(before.level, 2, "原始进度描述跳过路径，不应被推进");
    assert_eq!(after.level, 3);
}

#[test]
fn level_beyond_the_sequence_is_rejected() {
    let too_deep = u8::try_from(REVIEW_INTERVALS_DAYS.len()).expect("序列长度足够小") + 1;
    let error = advance_review(
        &progress(too_deep),
        date(2026, 11, 2),
        &REVIEW_INTERVALS_DAYS,
    )
    .expect_err("越界级别必须报错");
    assert!(matches!(error, ScheduleError::InvalidLevel { .. }));
}
