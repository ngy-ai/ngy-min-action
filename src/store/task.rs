//! 任务与频率的读写。

use rusqlite::{OptionalExtension, params};

use crate::model::{
    CategoryId, DomainError, Frequency, MAX_TASK_NAME_CHARS, ReviewProgress, Task, TaskDraft,
    TaskId, TaskMedia, TaskMediaId, TaskMediaKind, TaskStatus, validate_description,
    validate_estimated_minutes, validate_name,
};
use crate::store::action::map_progress;
use crate::store::{OpTrace, Store, decode_date, encode_date, now_rfc3339};

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
        let mut trace = OpTrace::begin("create_task");
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
                (category_id, name, freq_kind, freq_interval, estimated_minutes, status, created_at, description)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7)",
            params![
                category.0,
                name,
                draft.frequency.kind(),
                draft.frequency.raw_interval(),
                estimated,
                now_rfc3339(),
                draft.description,
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO review_progress (task_id, level, next_review_on, graduated, updated_at)
             VALUES (?1, 0, NULL, 0, ?2)",
            params![id, now_rfc3339()],
        )?;
        transaction.commit()?;
        let task = TaskId(id);
        trace.ok();
        Ok(task)
    }

    /// 列出分类下未删除的任务。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn tasks(&self, category: CategoryId) -> Result<Vec<Task>, DomainError> {
        let mut trace = OpTrace::begin("tasks");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                    status, last_generated_on, created_at, description
               FROM tasks
              WHERE category_id = ?1 AND deleted_at IS NULL
              ORDER BY id",
        )?;
        let rows = statement.query_map(params![category.0], map_task)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        trace.ok();
        Ok(tasks)
    }

    /// 列出所有未删除的任务（含暂停），供界面解析行动所属的任务与分类名称。
    ///
    /// 暂停只影响是否生成新行动，不影响既有行动的名称展示。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn all_tasks(&self) -> Result<Vec<Task>, DomainError> {
        let mut trace = OpTrace::begin("all_tasks");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                    status, last_generated_on, created_at, description
               FROM tasks
              WHERE deleted_at IS NULL
              ORDER BY id",
        )?;
        let rows = statement.query_map([], map_task)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        trace.ok();
        Ok(tasks)
    }

    /// 重命名任务。
    ///
    /// # Errors
    ///
    /// 名称非法或任务不存在时返回相应 [`DomainError`]。
    pub fn rename_task(&self, id: TaskId, name: &str) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("rename_task");
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
        trace.ok();
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
        let mut trace = OpTrace::begin("set_task_status");
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
        trace.ok();
        Ok(())
    }

    /// 更新任务的频率。
    ///
    /// 只改 `freq_kind` 与 `freq_interval`；名称、状态、`last_generated_on`
    /// 与复习进度都不变。已生成的行动不追溯，下一次生成按新间隔判定。
    ///
    /// # Errors
    ///
    /// 任务不存在或已删除时返回 [`DomainError::NotFound`]。
    pub fn update_task_frequency(
        &self,
        id: TaskId,
        frequency: Frequency,
    ) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("update_task_frequency");
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE tasks SET freq_kind = ?2, freq_interval = ?3
              WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, frequency.kind(), frequency.raw_interval()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 更新任务的预计耗时。
    ///
    /// 传 `None` 表示清空估时。只改 `estimated_minutes`；名称、频率、状态
    /// 与已生成的行动都不变。
    ///
    /// # Errors
    ///
    /// 任务不存在或已删除时返回 [`DomainError::NotFound`]。
    pub fn update_task_estimated_minutes(
        &self,
        id: TaskId,
        minutes: Option<u16>,
    ) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("update_task_estimated_minutes");
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE tasks SET estimated_minutes = ?2
              WHERE id = ?1 AND deleted_at IS NULL",
            params![id.0, minutes],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 某任务尚未结算的行动数量（`pending` 与 `deferred`）。
    ///
    /// 用于删除确认弹窗说明影响范围；已结算的行动不计入。任务不存在或已删除
    /// 时返回 0，不视为错误。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn task_open_actions(&self, task: TaskId) -> Result<usize, DomainError> {
        let mut trace = OpTrace::begin("task_open_actions");
        let connection = self.guard();
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM actions
              WHERE task_id = ?1 AND status IN ('pending', 'deferred')",
            params![task.0],
            |row| row.get(0),
        )?;
        trace.ok();
        Ok(usize::try_from(count).unwrap_or_default())
    }

    /// 软删除任务，其未完成行动转为过期。
    ///
    /// # Errors
    ///
    /// 任务不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_task(&self, id: TaskId) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("delete_task");
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
        trace.ok();
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

    /// 某分类下全部未删除任务的复习进度。
    ///
    /// 无复习记录的任务不出现在结果中，调用方据此显示「未开始」。
    /// 一次 JOIN 取回，避免列表渲染时逐条查询。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn progress_for_category(
        &self,
        category: CategoryId,
    ) -> Result<Vec<ReviewProgress>, DomainError> {
        let mut trace = OpTrace::begin("progress_for_category");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT rp.task_id, rp.level, rp.next_review_on, rp.graduated
               FROM review_progress rp
               JOIN tasks t ON t.id = rp.task_id
              WHERE t.category_id = ?1 AND t.deleted_at IS NULL
              ORDER BY rp.task_id",
        )?;
        let rows = statement.query_map(params![category.0], map_progress)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        trace.ok();
        Ok(items)
    }

    /// 按标识读取单个任务。
    ///
    /// # Errors
    ///
    /// 任务不存在或已删除时返回 [`DomainError::NotFound`]。
    pub fn task(&self, id: TaskId) -> Result<Task, DomainError> {
        let mut trace = OpTrace::begin("task");
        let connection = self.guard();
        let task = connection
            .query_row(
                "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                        status, last_generated_on, created_at, description
                   FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                params![id.0],
                map_task,
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "任务",
                id: id.0,
            })?;
        trace.ok();
        Ok(task)
    }

    /// 读取一个任务的内容。
    ///
    /// 无内容返回 `None`；任务不存在时返回 [`DomainError::NotFound`]。
    ///
    /// # Errors
    ///
    /// 任务不存在或查询失败时返回 [`DomainError`]。
    pub fn task_description(&self, task: TaskId) -> Result<Option<String>, DomainError> {
        let mut trace = OpTrace::begin("task_description");
        let connection = self.guard();
        let description = connection
            .query_row(
                "SELECT description FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                params![task.0],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "任务",
                id: task.0,
            })?;
        trace.ok();
        Ok(description)
    }

    /// 保存或清除一个任务的内容。
    ///
    /// 入参按 [`validate_description`] 规范化；空白内容写成 `NULL`，等价于清除。
    ///
    /// # Errors
    ///
    /// 内容超长或含非法控制字符时返回 [`DomainError::InvalidValue`]；任务不存在时返回
    /// [`DomainError::NotFound`]。
    pub fn set_task_description(&self, task: TaskId, description: &str) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("set_task_description");
        let normalized = validate_description(description)?;
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE tasks SET description = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![task.0, normalized],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: task.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 列出某任务的媒体附件，按加入顺序排列。
    ///
    /// # Errors
    ///
    /// 查询失败时返回 [`DomainError::Store`]。
    pub fn task_media(&self, task: TaskId) -> Result<Vec<TaskMedia>, DomainError> {
        let mut trace = OpTrace::begin("task_media");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, task_id, kind, file_name, original_name, byte_size, created_at
               FROM task_content_media WHERE task_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map(params![task.0], map_media)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        trace.ok();
        Ok(items)
    }

    /// 记录一段已经落盘的媒体附件。
    ///
    /// `file_name` 必须是单个文件名（不含 `/` 与 `\`），由媒体存储模块生成。
    ///
    /// # Errors
    ///
    /// 文件名不合法时返回 [`DomainError::InvalidValue`]；任务不存在时返回
    /// [`DomainError::NotFound`]。
    pub fn add_task_media(
        &self,
        task: TaskId,
        kind: TaskMediaKind,
        file_name: &str,
        original_name: &str,
        byte_size: u64,
    ) -> Result<TaskMedia, DomainError> {
        let mut trace = OpTrace::begin("add_task_media");
        if !is_single_component(file_name) {
            return Err(DomainError::InvalidValue {
                field: "媒体文件名",
                expected: "不含路径分隔符的单个文件名".to_owned(),
            });
        }
        let byte_size = i64::try_from(byte_size).map_err(|_| DomainError::InvalidValue {
            field: "媒体大小",
            expected: "不超过 i64 上限的字节数".to_owned(),
        })?;
        let connection = self.guard();
        let exists = connection
            .query_row(
                "SELECT 1 FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                params![task.0],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(DomainError::NotFound {
                entity: "任务",
                id: task.0,
            });
        }
        let original_name = sanitize_original_name(original_name);
        let created_at = now_rfc3339();
        connection.execute(
            "INSERT INTO task_content_media
                (task_id, kind, file_name, original_name, byte_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task.0,
                kind.as_str(),
                file_name,
                original_name,
                byte_size,
                created_at
            ],
        )?;
        let media = TaskMedia {
            id: TaskMediaId(connection.last_insert_rowid()),
            task_id: task,
            kind,
            file_name: file_name.to_owned(),
            original_name,
            byte_size: u64::try_from(byte_size).unwrap_or(0),
            created_at,
        };
        trace.ok();
        Ok(media)
    }

    /// 删除一段媒体附件的记录并返回它，供调用方清理磁盘文件。
    ///
    /// # Errors
    ///
    /// 记录不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_task_media(&self, media: TaskMediaId) -> Result<TaskMedia, DomainError> {
        let mut trace = OpTrace::begin("delete_task_media");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let media = transaction
            .query_row(
                "SELECT id, task_id, kind, file_name, original_name, byte_size, created_at
                   FROM task_content_media WHERE id = ?1",
                params![media.0],
                map_media,
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "媒体附件",
                id: media.0,
            })?;
        transaction.execute(
            "DELETE FROM task_content_media WHERE id = ?1",
            params![media.id.0],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(media)
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
    let description: Option<String> = row.get(9)?;
    Ok(Task {
        id: TaskId(row.get(0)?),
        category_id: CategoryId(row.get(1)?),
        name: row.get(2)?,
        frequency,
        estimated_minutes: row.get(5)?,
        description,
        status: TaskStatus::parse(&status),
        last_generated_on: last_generated.as_deref().and_then(decode_date),
        created_at: row.get(8)?,
    })
}

fn map_media(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskMedia> {
    let byte_size: i64 = row.get(5)?;
    Ok(TaskMedia {
        id: TaskMediaId(row.get(0)?),
        task_id: TaskId(row.get(1)?),
        kind: TaskMediaKind::parse(&row.get::<_, String>(2)?),
        file_name: row.get(3)?,
        original_name: row.get(4)?,
        byte_size: u64::try_from(byte_size).unwrap_or(0),
        created_at: row.get(6)?,
    })
}

fn is_single_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

fn sanitize_original_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|character| !character.is_control())
        .take(200)
        .collect();
    if cleaned.trim().is_empty() {
        "媒体".to_owned()
    } else {
        cleaned
    }
}
