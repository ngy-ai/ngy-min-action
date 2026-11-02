//! 建表与迁移。
//!
//! 表的约束与 `specs/001-min-action-core/data-model.md` 一一对应；不在这里写业务规则，
//! 业务规则在 `model` 与 `store` 的方法里。

use rusqlite::Connection;

/// 当前期望的 schema 版本。
pub(crate) const SCHEMA_VERSION: i64 = 1;

/// 建表语句。
///
/// 三张实体表使用软删除（`deleted_at`），因此唯一性用**部分索引**表达：
/// 只约束未删除的行，避免删掉「口语」之后无法再建同名分类。
const DDL: &str = r"
CREATE TABLE IF NOT EXISTS goals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    description TEXT,
    created_at  TEXT    NOT NULL,
    deleted_at  TEXT
);

CREATE TABLE IF NOT EXISTS categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_id    INTEGER NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    created_at TEXT    NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_live_name
    ON categories(goal_id, name) WHERE deleted_at IS NULL;

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
    UNIQUE(task_id, on_date)
);

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

/// 应用建表语句并记录 schema 版本。
pub(crate) fn apply(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(DDL)?;
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}
