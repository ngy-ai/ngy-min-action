//! 任务与频率的读写。

use rusqlite::{OptionalExtension, params};

use crate::model::{
    CategoryId, DomainError, MAX_TASK_NAME_CHARS, Task, TaskDraft, TaskId, TaskStatus,
    validate_estimated_minutes, validate_name,
};
use crate::store::{Store, decode_date, encode_date, now_rfc3339};

impl Store {
    /// 在分类下创建任务。
    ///
    /// # Errors
    ///
    /// 名称非法、频率越界或分类不存在时返回相应 [`DomainError`]。
    pub fn create_task(
        &self,
        category: CategoryId,
        draft: TaskDraft,
    ) -> Result<TaskId, DomainError> {
        let name = validate_name(&draft.name, "任务名称", MAX_TASK_NAME_CHARS)?;
        let estimated = validate_estimated_minutes(draft.estimated_minutes)?;

        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let exists: Option<i64> = transaction
            .query_row(
                "SELECT id FROM categories WHERE id = ?1 AND deleted_at IS NULL",
                params![category.0],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(DomainError::NotFound {
                entity: "分类",
                id: category.0,
            });
        }

        transaction.execute(
            "INSERT INTO tasks
                (category_id, name, freq_kind, freq_interval, estimated_minutes, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6)",
            params![
                category.0,
                name,
                draft.frequency.kind(),
                draft.frequency.raw_interval(),
                estimated,
                now_rfc3339()
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO review_progress (task_id, level, next_review_on, graduated, updated_at)
             VALUES (?1, 0, NULL, 0, ?2)",
            params![id, now_rfc3339()],
        )?;
        transaction.commit()?;
        Ok(TaskId(id))
    }

    /// 列出分类下未删除的任务。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn tasks(&self, category: CategoryId) -> Result<Vec<Task>, DomainError> {
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                    status, last_generated_on, created_at
               FROM tasks
              WHERE category_id = ?1 AND deleted_at IS NULL
              ORDER BY id",
        )?;
        let rows = statement.query_map(params![category.0], map_task)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// 列出所有未删除且未暂停的任务，供每日生成使用。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn active_tasks(&self) -> Result<Vec<Task>, DomainError> {
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                    status, last_generated_on, created_at
               FROM tasks
              WHERE deleted_at IS NULL AND status = 'active'
              ORDER BY id",
        )?;
        let rows = statement.query_map([], map_task)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// 重命名任务。
    ///
    /// # Errors
    ///
    /// 名称非法或任务不存在时返回相应 [`DomainError`]。
    pub fn rename_task(&self, id: TaskId, name: &str) -> Result<(), DomainError> {
        let name = validate_name(name, "任务名称", MAX_TASK_NAME_CHARS)?;
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE tasks SET name = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, name],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            });
        }
        Ok(())
    }

    /// 暂停或恢复任务。
    ///
    /// 恢复后不补生成暂停期间的行动：`last_generated_on` 保持原值。
    ///
    /// # Errors
    ///
    /// 任务不存在时返回 [`DomainError::NotFound`]。
    pub fn set_task_status(&self, id: TaskId, status: TaskStatus) -> Result<(), DomainError> {
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE tasks SET status = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, status.as_str()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            });
        }
        Ok(())
    }

    /// 软删除任务，其未完成行动转为过期。
    ///
    /// # Errors
    ///
    /// 任务不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_task(&self, id: TaskId) -> Result<(), DomainError> {
        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let updated = transaction.execute(
            "UPDATE tasks SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            });
        }
        transaction.execute(
            "UPDATE actions SET status = 'expired'
              WHERE task_id = ?1 AND status IN ('pending', 'deferred')",
            params![id.0],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// 记录某任务最近一次生成常规行动的日期。
    pub(crate) fn touch_task_generated_on(
        connection: &rusqlite::Connection,
        task: TaskId,
        date: chrono::NaiveDate,
    ) -> Result<(), DomainError> {
        connection.execute(
            "UPDATE tasks SET last_generated_on = ?2 WHERE id = ?1",
            params![task.0, encode_date(date)],
        )?;
        Ok(())
    }
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let kind: String = row.get(3)?;
    let interval: i64 = row.get(4)?;
    // 入库值越界说明数据被外部改坏：退化为每天，保证界面仍可用而不是整页报错。
    let frequency = crate::model::Frequency::from_parts(&kind, interval)
        .unwrap_or(crate::model::Frequency::Daily);
    let status: String = row.get(6)?;
    let last_generated: Option<String> = row.get(7)?;
    Ok(Task {
        id: TaskId(row.get(0)?),
        category_id: CategoryId(row.get(1)?),
        name: row.get(2)?,
        frequency,
        estimated_minutes: row.get(5)?,
        status: TaskStatus::parse(&status),
        last_generated_on: last_generated.as_deref().and_then(decode_date),
        created_at: row.get(8)?,
    })
}
