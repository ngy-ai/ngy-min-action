//! 建表与迁移。
//!
//! 表的约束与 `specs/002-name-uniqueness/data-model.md` 一一对应；不在这里写业务
//! 规则，业务规则在 `model` 与 `store` 的方法里。

use std::time::Instant;

use rusqlite::Connection;
use tracing::info;

use crate::common::elapsed_ms;

/// 当前期望的 schema 版本。
///
/// 1 → 2 的变更：目标与分类增加 `name_folded` 列，唯一性改由整表唯一索引承担。
/// 2 → 3 的变更：行动增加 `note` 列，新增 `action_images` 表；保留全部业务数据。
pub(crate) const SCHEMA_VERSION: i64 = 3;

/// 建表语句。
///
/// 形态以 `specs/002-name-uniqueness/data-model.md` 为准：名称判重落在
/// `name_folded` 上的**整表**唯一索引，不带 `deleted_at IS NULL` 条件——
/// 软删除不释放名称。
const DDL: &str = r"
CREATE TABLE IF NOT EXISTS goals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    name_folded TEXT    NOT NULL,
    description TEXT,
    created_at  TEXT    NOT NULL,
    deleted_at  TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_goals_name_folded
    ON goals(name_folded);

CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_id     INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    name_folded TEXT    NOT NULL,
    created_at  TEXT    NOT NULL,
    deleted_at  TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_goal_name_folded
    ON categories(goal_id, name_folded);

CREATE TABLE IF NOT EXISTS tasks (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id       INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    name              TEXT    NOT NULL,
    freq_kind         TEXT    NOT NULL,
    freq_interval     INTEGER NOT NULL,
    estimated_minutes INTEGER,
    status            TEXT    NOT NULL DEFAULT 'active',
    last_generated_on TEXT,
    created_at        TEXT    NOT NULL,
    deleted_at        TEXT
);

CREATE TABLE IF NOT EXISTS actions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id      INTEGER NOT NULL REFERENCES tasks(id),
    on_date      TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    review_level INTEGER,
    status       TEXT    NOT NULL,
    seq          INTEGER NOT NULL,
    created_at   TEXT    NOT NULL,
    settled_at   TEXT,
    note         TEXT,
    UNIQUE(task_id, on_date)
);

CREATE TABLE IF NOT EXISTS action_images (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id     INTEGER NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    file_name     TEXT    NOT NULL,
    original_name TEXT    NOT NULL,
    byte_size     INTEGER NOT NULL,
    created_at    TEXT    NOT NULL,
    UNIQUE(action_id, file_name)
);

CREATE INDEX IF NOT EXISTS idx_action_images_action
    ON action_images(action_id);

CREATE TABLE IF NOT EXISTS review_progress (
    task_id        INTEGER PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
    level          INTEGER NOT NULL DEFAULT 0,
    next_review_on TEXT,
    graduated      INTEGER NOT NULL DEFAULT 0,
    updated_at     TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS action_settlements (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id          INTEGER NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    prev_status        TEXT    NOT NULL,
    prev_level         INTEGER NOT NULL,
    prev_next_review_on TEXT,
    prev_graduated     INTEGER NOT NULL,
    settled_at         TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS reminder_state (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    last_notified_on TEXT,
    last_viewed_on   TEXT
);

INSERT OR IGNORE INTO reminder_state (id) VALUES (1);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// 升级旧库时的清除顺序。
///
/// `foreign_keys=ON` 时父表不能先于子表删除，顺序颠倒会触发外键错误。
const DROP_TABLES: &str = r"
DROP TABLE IF EXISTS action_images;
DROP TABLE IF EXISTS action_settlements;
DROP TABLE IF EXISTS actions;
DROP TABLE IF EXISTS review_progress;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS goals;
DROP TABLE IF EXISTS reminder_state;
DROP TABLE IF EXISTS settings;
";

/// 建表与迁移的错误。
#[derive(Debug)]
pub(crate) enum SchemaError {
    /// SQL 执行失败。
    Sql(rusqlite::Error),
    /// 库的架构版本高于程序支持的版本。
    Newer {
        /// 库里的版本。
        found: i64,
        /// 程序支持的版本。
        supported: i64,
    },
}

impl From<rusqlite::Error> for SchemaError {
    fn from(error: rusqlite::Error) -> Self {
        SchemaError::Sql(error)
    }
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::Sql(error) => write!(formatter, "{error}"),
            SchemaError::Newer { found, supported } => write!(
                formatter,
                "数据库架构版本 {found} 高于程序支持的 {supported}，请升级程序，或换回产生该库的版本"
            ),
        }
    }
}

impl std::error::Error for SchemaError {}

/// 按架构版本建表或升级。
///
/// # Errors
///
/// 建表或升级的 SQL 失败时返回 [`SchemaError::Sql`]；库的版本高于
/// [`SCHEMA_VERSION`] 时返回 [`SchemaError::Newer`]，避免把新库按旧结构解读。
pub(crate) fn apply(connection: &Connection) -> Result<(), SchemaError> {
    let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match current {
        0 => {
            connection.execute_batch(DDL)?;
            connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        SCHEMA_VERSION => {}
        from if from < SCHEMA_VERSION => migrate(connection, from)?,
        found => {
            return Err(SchemaError::Newer {
                found,
                supported: SCHEMA_VERSION,
            });
        }
    }
    Ok(())
}

/// 2 → 3 的增量升级语句。
///
/// 只做加列与建表，不触碰任何既有行；结构与 [`DDL`] 中对应片段保持一致。
const V3_UPGRADE: &str = r"
ALTER TABLE actions ADD COLUMN note TEXT;

CREATE TABLE IF NOT EXISTS action_images (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id     INTEGER NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    file_name     TEXT    NOT NULL,
    original_name TEXT    NOT NULL,
    byte_size     INTEGER NOT NULL,
    created_at    TEXT    NOT NULL,
    UNIQUE(action_id, file_name)
);

CREATE INDEX IF NOT EXISTS idx_action_images_action
    ON action_images(action_id);
";

/// 把旧库升级到当前结构。
///
/// 2 → 3 保留全部业务数据；更早的版本缺少名称折叠列，无法原地补齐，仍按原有策略
/// 清空重建。迁移必须整体成功：任一步失败都不改变库里的任何一个字节。
fn migrate(connection: &Connection, from: i64) -> Result<(), SchemaError> {
    if from == 2 {
        return upgrade_v2_to_v3(connection);
    }
    rebuild(connection, from)
}

fn upgrade_v2_to_v3(connection: &Connection) -> Result<(), SchemaError> {
    let started = Instant::now();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(V3_UPGRADE)?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    info!(
        op = "store_migrate",
        from = 2,
        to = SCHEMA_VERSION,
        result = "ok",
        elapsed_ms = elapsed_ms(started),
        "旧库已增量升级，业务数据保留"
    );
    Ok(())
}

fn rebuild(connection: &Connection, from: i64) -> Result<(), SchemaError> {
    let started = Instant::now();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(DROP_TABLES)?;
    transaction.execute_batch(DDL)?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    info!(
        op = "store_migrate",
        from,
        to = SCHEMA_VERSION,
        result = "ok",
        elapsed_ms = elapsed_ms(started),
        "旧库已清空并按新结构重建"
    );
    Ok(())
}
