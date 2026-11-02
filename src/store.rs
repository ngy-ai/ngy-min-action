//! 存储层。
//!
//! 单一 SQLite 连接由 [`Store`] 串行化持有；每个写操作在自己的事务里完成，
//! 失败整笔回滚，不留半完成状态。

mod action;
mod goal;
mod reminder;
mod schema;
mod task;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{NaiveDate, Utc};
use rusqlite::Connection;

use crate::model::DomainError;

/// 当前时刻的 RFC 3339 表示，用于各表的 `created_at` / `settled_at`。
pub(crate) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// 把本地日期编码成入库用的 `YYYY-MM-DD`。
pub(crate) fn encode_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// 从 `YYYY-MM-DD` 还原本地日期；无法解析时返回 `None`，由调用方决定如何报错。
pub(crate) fn decode_date(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
}

/// 存储层错误。文案统一指向「出了什么问题 / 怎么修」。
#[derive(Debug)]
pub enum StoreError {
    /// 无法打开或创建数据库文件。
    Open { path: String, reason: String },
    /// 初始化表结构失败。
    Migrate { reason: String },
    /// 读写失败。
    Query { reason: String },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Open { path, reason } => write!(
                formatter,
                "无法打开数据库 {path}：{reason}。请确认该目录可写，或删除损坏的数据文件后重试"
            ),
            StoreError::Migrate { reason } => write!(
                formatter,
                "数据库结构初始化失败：{reason}。请备份数据文件后重试"
            ),
            StoreError::Query { reason } => write!(
                formatter,
                "数据库读写失败：{reason}。请重试；若持续失败，请备份并重建数据文件"
            ),
        }
    }
}

impl std::error::Error for StoreError {}

impl StoreError {
    pub(crate) fn query(error: &rusqlite::Error) -> Self {
        StoreError::Query {
            reason: error.to_string(),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::query(&error)
    }
}

impl From<rusqlite::Error> for DomainError {
    fn from(error: rusqlite::Error) -> Self {
        DomainError::Store(StoreError::query(&error))
    }
}

/// SQLite 存储门面。
#[derive(Debug, Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Store {
    /// 打开或创建数据库，并完成建表。
    ///
    /// # Errors
    ///
    /// 目录不可写、文件无法打开或建表失败时返回 [`StoreError`]。
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| StoreError::Open {
                path: path.display().to_string(),
                reason: format!("无法创建目录 {}：{error}", parent.display()),
            })?;
        }

        let connection = Connection::open(path).map_err(|error| StoreError::Open {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;

        let journal: String = connection
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .map_err(|error| StoreError::Migrate {
                reason: format!("启用 WAL 失败：{error}"),
            })?;
        debug_assert_eq!(journal.to_lowercase(), "wal");

        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| StoreError::Migrate {
                reason: format!("启用外键约束失败：{error}"),
            })?;

        schema::apply(&connection).map_err(|error| StoreError::Migrate {
            reason: error.to_string(),
        })?;

        Ok(Store {
            connection: Arc::new(Mutex::new(connection)),
            path: path.to_path_buf(),
        })
    }

    /// 数据库文件路径。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn guard(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// 判断错误是否由唯一约束冲突引起。
pub(crate) fn is_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::ConstraintViolation
    )
}
