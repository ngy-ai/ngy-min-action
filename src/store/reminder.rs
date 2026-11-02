//! 提示记录与用户设置的读写。

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, params};

use crate::common::{DAILY_CAP_DEFAULT, DAILY_CAP_MAX, DAILY_CAP_MIN};
use crate::model::{DomainError, ReminderState};
use crate::store::{Store, decode_date, encode_date};

/// `settings` 表中存放每日上限的键。
const DAILY_CAP_KEY: &str = "daily_cap";

impl Store {
    /// 读取提示记录。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn reminder_state(&self) -> Result<ReminderState, DomainError> {
        let connection = self.guard();
        let row: Option<(Option<String>, Option<String>)> = connection
            .query_row(
                "SELECT last_notified_on, last_viewed_on FROM reminder_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (notified, viewed) = row.unwrap_or((None, None));
        Ok(ReminderState {
            last_notified_on: notified.as_deref().and_then(decode_date),
            last_viewed_on: viewed.as_deref().and_then(decode_date),
        })
    }

    /// 记录当天已经弹出过提示。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn mark_notified(&self, today: NaiveDate) -> Result<(), DomainError> {
        let connection = self.guard();
        connection.execute(
            "UPDATE reminder_state SET last_notified_on = ?1 WHERE id = 1",
            params![encode_date(today)],
        )?;
        Ok(())
    }

    /// 记录用户当天查看过今日清单。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn mark_viewed(&self, today: NaiveDate) -> Result<(), DomainError> {
        let connection = self.guard();
        connection.execute(
            "UPDATE reminder_state SET last_viewed_on = ?1 WHERE id = 1",
            params![encode_date(today)],
        )?;
        Ok(())
    }

    /// 当前每日行动上限。
    ///
    /// 未配置或配置损坏时回落到默认值，保证界面可用。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn daily_cap(&self) -> Result<u8, DomainError> {
        let connection = self.guard();
        let raw: Option<String> = connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![DAILY_CAP_KEY],
                |row| row.get(0),
            )
            .optional()?;
        let cap = raw
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(DAILY_CAP_DEFAULT)
            .clamp(DAILY_CAP_MIN, DAILY_CAP_MAX);
        Ok(cap)
    }

    /// 设置每日行动上限。
    ///
    /// # Errors
    ///
    /// 超出 `DAILY_CAP_MIN..=DAILY_CAP_MAX` 时返回 [`DomainError::InvalidValue`]。
    pub fn set_daily_cap(&self, cap: u8) -> Result<(), DomainError> {
        if !(DAILY_CAP_MIN..=DAILY_CAP_MAX).contains(&cap) {
            return Err(DomainError::InvalidValue {
                field: "每日上限",
                expected: format!("{DAILY_CAP_MIN} 到 {DAILY_CAP_MAX} 之间的整数"),
            });
        }
        let connection = self.guard();
        connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![DAILY_CAP_KEY, cap.to_string()],
        )?;
        Ok(())
    }
}
