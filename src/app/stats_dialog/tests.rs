//! 统计对话框内容整理逻辑无头验收（T014）。
//!
//! 核心是 `stats_rows`：把 `ReviewStats` 整理成「标签-值」行。直接验证其输出，无需 GPU。

use std::collections::HashMap;

use super::{STATS_WINDOW_DAYS, stats_rows};
use crate::model::{ActionTally, ReviewStats};

/// 一个已知样本，便于断言具体数值。
fn sample() -> ReviewStats {
    ReviewStats {
        window_days: STATS_WINDOW_DAYS,
        tally: ActionTally {
            done: 7,
            skipped: 2,
            expired: 1,
        },
        current_streak: 3,
        longest_streak: 10,
        untouched_tasks: 4,
        learning_tasks: 5,
        graduated_tasks: 6,
    }
}

#[test]
fn rows_include_window_and_tallies() {
    let map: HashMap<&str, String> = stats_rows(&sample()).into_iter().collect();
    assert_eq!(map.get("统计窗口"), Some(&"近 30 天".to_string()));
    assert_eq!(map.get("完成"), Some(&"7".to_string()));
    assert_eq!(map.get("跳过"), Some(&"2".to_string()));
    assert_eq!(map.get("过期"), Some(&"1".to_string()));
    assert_eq!(map.get("当前连续达成"), Some(&"3 天".to_string()));
    assert_eq!(map.get("历史最长连续"), Some(&"10 天".to_string()));
    assert_eq!(map.get("未开始任务"), Some(&"4".to_string()));
    assert_eq!(map.get("复习中任务"), Some(&"5".to_string()));
    assert_eq!(map.get("已巩固任务"), Some(&"6".to_string()));
}

#[test]
fn completion_percent_is_rounded() {
    // 7 / (7 + 2 + 1) = 70%
    let map: HashMap<&str, String> = stats_rows(&sample()).into_iter().collect();
    assert_eq!(map.get("完成率"), Some(&"70%".to_string()));
}

#[test]
fn zero_settled_shows_dash() {
    let stats = ReviewStats {
        tally: ActionTally {
            done: 0,
            skipped: 0,
            expired: 0,
        },
        ..sample()
    };
    let map: HashMap<&str, String> = stats_rows(&stats).into_iter().collect();
    assert_eq!(map.get("完成率"), Some(&"—".to_string()));
}
