//! 目标与分类的读写。

use rusqlite::{OptionalExtension, params};

use crate::model::{
    Category, CategoryId, DeletionImpact, DomainError, Goal, GoalId, MAX_CATEGORY_NAME,
    MAX_DESCRIPTION, MAX_GOAL_NAME, validate_name,
};
use crate::store::{OpTrace, Store, is_unique_violation, now_rfc3339};

impl Store {
    /// 创建目标。
    ///
    /// # Errors
    ///
    /// 名称为空、超长或含控制字符时返回 [`DomainError::InvalidName`]。
    pub fn create_goal(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<GoalId, DomainError> {
        let mut trace = OpTrace::begin("create_goal");
        let name = validate_name(name, "目标名称", MAX_GOAL_NAME)?;
        let description = normalize_description(description)?;
        let connection = self.guard();
        connection.execute(
            "INSERT INTO goals (name, description, created_at) VALUES (?1, ?2, ?3)",
            params![name, description, now_rfc3339()],
        )?;
        let id = GoalId(connection.last_insert_rowid());
        trace.ok();
        Ok(id)
    }

    /// 列出所有未删除的目标。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn goals(&self) -> Result<Vec<Goal>, DomainError> {
        let mut trace = OpTrace::begin("goals");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, name, description, created_at FROM goals
             WHERE deleted_at IS NULL ORDER BY id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Goal {
                id: GoalId(row.get(0)?),
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut goals = Vec::new();
        for row in rows {
            goals.push(row?);
        }
        trace.ok();
        Ok(goals)
    }

    /// 重命名目标。
    ///
    /// # Errors
    ///
    /// 名称为空或超长时返回 [`DomainError::InvalidName`]；目标不存在时返回
    /// [`DomainError::NotFound`]。
    pub fn rename_goal(&self, id: GoalId, name: &str) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("rename_goal");
        let name = validate_name(name, "目标名称", MAX_GOAL_NAME)?;
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE goals SET name = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, name],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "目标",
                id: id.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 软删除目标，返回将被一并影响的范围。
    ///
    /// 已完成的行动与结算记录全部保留，供历史查询。
    ///
    /// # Errors
    ///
    /// 目标不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_goal(&self, id: GoalId) -> Result<DeletionImpact, DomainError> {
        let mut trace = OpTrace::begin("delete_goal");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let impact = goal_impact(&transaction, id.0)?;
        let stamp = now_rfc3339();
        let updated = transaction.execute(
            "UPDATE goals SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, stamp],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "目标",
                id: id.0,
            });
        }
        transaction.execute(
            "UPDATE categories SET deleted_at = ?2 WHERE goal_id = ?1",
            params![id.0, stamp],
        )?;
        transaction.execute(
            "UPDATE tasks SET deleted_at = ?2
             WHERE category_id IN (SELECT id FROM categories WHERE goal_id = ?1)",
            params![id.0, stamp],
        )?;
        transaction.execute(
            "UPDATE actions SET status = 'expired'
             WHERE status IN ('pending', 'deferred') AND task_id IN (
                 SELECT t.id FROM tasks t
                 JOIN categories c ON t.category_id = c.id
                 WHERE c.goal_id = ?1)",
            params![id.0],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(impact)
    }

    /// 在目标下创建分类。
    ///
    /// # Errors
    ///
    /// 同一目标下重名时返回 [`DomainError::DuplicateName`]；目标不存在时返回
    /// [`DomainError::NotFound`]。
    pub fn create_category(&self, goal: GoalId, name: &str) -> Result<CategoryId, DomainError> {
        let mut trace = OpTrace::begin("create_category");
        let name = validate_name(name, "分类名称", MAX_CATEGORY_NAME)?;
        let connection = self.guard();
        ensure_goal_exists(&connection, goal.0)?;
        let outcome = match connection.execute(
            "INSERT INTO categories (goal_id, name, created_at) VALUES (?1, ?2, ?3)",
            params![goal.0, name, now_rfc3339()],
        ) {
            Ok(_) => Ok(CategoryId(connection.last_insert_rowid())),
            Err(error) if is_unique_violation(&error) => Err(DomainError::DuplicateName {
                scope: "同一目标下的分类",
                name,
            }),
            Err(error) => Err(error.into()),
        };
        if outcome.is_ok() {
            trace.ok();
        }
        outcome
    }

    /// 列出目标下未删除的分类。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn categories(&self, goal: GoalId) -> Result<Vec<Category>, DomainError> {
        let mut trace = OpTrace::begin("categories");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, goal_id, name, created_at FROM categories
             WHERE goal_id = ?1 AND deleted_at IS NULL ORDER BY id",
        )?;
        let rows = statement.query_map(params![goal.0], |row| {
            Ok(Category {
                id: CategoryId(row.get(0)?),
                goal_id: GoalId(row.get(1)?),
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut categories = Vec::new();
        for row in rows {
            categories.push(row?);
        }
        trace.ok();
        Ok(categories)
    }

    /// 列出所有未删除的分类，供界面把行动映射成「分类 · 任务」。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn all_categories(&self) -> Result<Vec<Category>, DomainError> {
        let mut trace = OpTrace::begin("all_categories");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, goal_id, name, created_at FROM categories
             WHERE deleted_at IS NULL ORDER BY goal_id, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Category {
                id: CategoryId(row.get(0)?),
                goal_id: GoalId(row.get(1)?),
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut categories = Vec::new();
        for row in rows {
            categories.push(row?);
        }
        trace.ok();
        Ok(categories)
    }

    /// 重命名分类。
    ///
    /// # Errors
    ///
    /// 同目标下重名返回 [`DomainError::DuplicateName`]；分类不存在返回
    /// [`DomainError::NotFound`]。
    pub fn rename_category(&self, id: CategoryId, name: &str) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("rename_category");
        let name = validate_name(name, "分类名称", MAX_CATEGORY_NAME)?;
        let connection = self.guard();
        let outcome = match connection.execute(
            "UPDATE categories SET name = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, name],
        ) {
            Ok(0) => Err(DomainError::NotFound {
                entity: "分类",
                id: id.0,
            }),
            Ok(_) => Ok(()),
            Err(error) if is_unique_violation(&error) => Err(DomainError::DuplicateName {
                scope: "同一目标下的分类",
                name,
            }),
            Err(error) => Err(error.into()),
        };
        if outcome.is_ok() {
            trace.ok();
        }
        outcome
    }

    /// 软删除分类。
    ///
    /// # Errors
    ///
    /// 分类不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_category(&self, id: CategoryId) -> Result<DeletionImpact, DomainError> {
        let mut trace = OpTrace::begin("delete_category");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let impact = category_impact(&transaction, id.0)?;
        let stamp = now_rfc3339();
        let updated = transaction.execute(
            "UPDATE categories SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, stamp],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "分类",
                id: id.0,
            });
        }
        transaction.execute(
            "UPDATE tasks SET deleted_at = ?2 WHERE category_id = ?1",
            params![id.0, stamp],
        )?;
        transaction.execute(
            "UPDATE actions SET status = 'expired'
             WHERE status IN ('pending', 'deferred')
               AND task_id IN (SELECT id FROM tasks WHERE category_id = ?1)",
            params![id.0],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(impact)
    }
}

fn normalize_description(description: Option<&str>) -> Result<Option<String>, DomainError> {
    let Some(raw) = description else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_DESCRIPTION {
        return Err(DomainError::InvalidValue {
            field: "目标描述",
            expected: format!("不超过 {MAX_DESCRIPTION} 个字符"),
        });
    }
    Ok(Some(trimmed.to_owned()))
}

fn ensure_goal_exists(connection: &rusqlite::Connection, id: i64) -> Result<(), DomainError> {
    let found: Option<i64> = connection
        .query_row(
            "SELECT id FROM goals WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    if found.is_none() {
        return Err(DomainError::NotFound {
            entity: "目标", id
        });
    }
    Ok(())
}

fn goal_impact(
    connection: &rusqlite::Transaction<'_>,
    goal: i64,
) -> Result<DeletionImpact, DomainError> {
    let impact = connection.query_row(
        "SELECT
            (SELECT COUNT(*) FROM categories WHERE goal_id = ?1 AND deleted_at IS NULL),
            (SELECT COUNT(*) FROM tasks t
               JOIN categories c ON t.category_id = c.id
              WHERE c.goal_id = ?1 AND t.deleted_at IS NULL),
            (SELECT COUNT(*) FROM actions a
               JOIN tasks t ON a.task_id = t.id
               JOIN categories c ON t.category_id = c.id
              WHERE c.goal_id = ?1 AND a.status IN ('pending', 'deferred'))",
        params![goal],
        map_impact,
    )?;
    Ok(impact)
}

fn category_impact(
    connection: &rusqlite::Transaction<'_>,
    category: i64,
) -> Result<DeletionImpact, DomainError> {
    let impact = connection.query_row(
        "SELECT
            0,
            (SELECT COUNT(*) FROM tasks WHERE category_id = ?1 AND deleted_at IS NULL),
            (SELECT COUNT(*) FROM actions a
               JOIN tasks t ON a.task_id = t.id
              WHERE t.category_id = ?1 AND a.status IN ('pending', 'deferred'))",
        params![category],
        map_impact,
    )?;
    Ok(impact)
}

fn map_impact(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeletionImpact> {
    let categories: i64 = row.get(0)?;
    let tasks: i64 = row.get(1)?;
    let pending_actions: i64 = row.get(2)?;
    Ok(DeletionImpact {
        categories: usize::try_from(categories).unwrap_or(usize::MAX),
        tasks: usize::try_from(tasks).unwrap_or(usize::MAX),
        pending_actions: usize::try_from(pending_actions).unwrap_or(usize::MAX),
    })
}
