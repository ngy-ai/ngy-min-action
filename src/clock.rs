//! 时间来源抽象。
//!
//! 业务层禁止直接调用 `chrono::Local::now()`：跨日切换、遗忘曲线推进与提示去重都无法
//! 用真实时间稳定测试，因此统一从 [`Clock`] 取「今天」。

use std::sync::Mutex;

use chrono::{Local, NaiveDate};

/// 「今天是哪一天」的来源。
pub trait Clock: Send + Sync {
    /// 返回本地日期。
    fn today(&self) -> NaiveDate;
}

/// 生产实现：读取系统本地时间。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}

/// 测试实现：日期可被显式推进，用于覆盖跨日与多日场景。
#[derive(Debug)]
pub struct FixedClock {
    today: Mutex<NaiveDate>,
}

impl FixedClock {
    /// 以给定日期为起点创建时钟。
    pub fn new(start: NaiveDate) -> Self {
        FixedClock {
            today: Mutex::new(start),
        }
    }

    /// 把日期推进若干天。
    pub fn advance_days(&self, days: i64) {
        let mut guard = self
            .today
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard += chrono::Duration::days(days);
    }

    /// 直接设置为某一天。
    pub fn set(&self, date: NaiveDate) {
        let mut guard = self
            .today
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = date;
    }
}

impl Clock for FixedClock {
    fn today(&self) -> NaiveDate {
        *self
            .today
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
