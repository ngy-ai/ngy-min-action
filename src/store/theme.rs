//! 主题偏好的持久化：复用 `settings` 表。

use rusqlite::{OptionalExtension, params};

use crate::model::{DomainError, ThemeChoice};
use crate::store::{OpTrace, Store};

/// `settings` 表中存放主题偏好的键。
const THEME_KEY: &str = "theme";

impl Store {
    /// 读取主题偏好。
    ///
    /// 无记录、空串或无法识别的值都返回 [`ThemeChoice::System`]，且不自动写库。
    ///
    /// # Errors
    ///
    /// 查询失败时返回 [`DomainError::Store`]。
    pub fn theme_choice(&self) -> Result<ThemeChoice, DomainError> {
        let mut trace = OpTrace::begin("theme_choice");
        let connection = self.guard();
        let raw: Option<String> = connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![THEME_KEY],
                |row| row.get(0),
            )
            .optional()?;
        let choice = raw
            .map(|value| {
                let parsed = ThemeChoice::parse(&value);
                if value.trim() != parsed.as_str() {
                    tracing::warn!(
                        op = "theme_choice",
                        result = "unknown_value",
                        "主题偏好取值无法识别，已回退跟随系统"
                    );
                }
                parsed
            })
            .unwrap_or_default();
        trace.ok();
        Ok(choice)
    }

    /// 写入主题偏好。
    ///
    /// 取值由 [`ThemeChoice`] 保证合法，参数化写入；不改变其它设置或任何业务行。
    ///
    /// # Errors
    ///
    /// 写入失败时返回 [`DomainError::Store`]。
    pub fn set_theme_choice(&self, choice: ThemeChoice) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("set_theme_choice");
        let connection = self.guard();
        connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![THEME_KEY, choice.as_str()],
        )?;
        trace.ok();
        Ok(())
    }
}
