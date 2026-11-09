//! 行动、每日生成与结算。

use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};

use crate::common::{DEFERRED_RETENTION_DAYS, REVIEW_INTERVALS_DAYS};
use crate::model::{
    Action, ActionId, ActionKind, ActionStatus, DomainError, HistoryPage, ReviewProgress, Task,
    TaskId, TaskStatus,
};
use crate::schedule::{PlanInput, advance_review, plan_day};
use crate::store::{OpTrace, Store, decode_date, encode_date, now_rfc3339};

impl Store {
    /// 列出某一天的全部行动，按展示顺序排列。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn actions_on(&self, today: NaiveDate) -> Result<Vec<Action>, DomainError> {
        let mut trace = OpTrace::begin("actions_on");
        let connection = self.guard();
        let actions = load_actions(&connection, today)?;
        trace.ok();
        Ok(actions)
    }

    /// 历史待办的一页，按日期倒序、同日按顺序倒序。
    ///
    /// `cursor` 取上一页返回的 [`HistoryPage::next_cursor`]，首屏传 `None`。
    /// `limit` 为 0 时按 1 处理，保证每次至少前进一行。
    ///
    /// # Errors
    ///
    /// 查询失败时返回 [`DomainError::Store`]。
    pub fn actions_page(
        &self,
        today: NaiveDate,
        cursor: Option<(NaiveDate, i64)>,
        limit: u16,
    ) -> Result<HistoryPage, DomainError> {
        let mut trace = OpTrace::begin("actions_page");
        let connection = self.guard();
        let limit = limit.max(1);
        let (cursor_date, cursor_seq) = match cursor {
            Some((date, seq)) => (Some(encode_date(date)), seq),
            None => (None, i64::MAX),
        };
        let mut statement = connection.prepare(
            "SELECT id, task_id, on_date, kind, review_level, status, seq, note
               FROM actions
              WHERE on_date < ?1
                AND (?2 IS NULL OR on_date < ?2 OR (on_date = ?2 AND seq < ?3))
              ORDER BY on_date DESC, seq DESC, id DESC
              LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![
                encode_date(today),
                cursor_date,
                cursor_seq,
                i64::from(limit) + 1
            ],
            map_action,
        )?;
        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        let has_more = actions.len() > usize::from(limit);
        if has_more {
            actions.truncate(usize::from(limit));
        }
        let next_cursor = if has_more {
            actions
                .last()
                .map(|action| (action.on_date, i64::from(action.seq)))
        } else {
            None
        };
        trace.ok();
        Ok(HistoryPage {
            actions,
            next_cursor,
            has_more,
        })
    }

    /// 生成某一天的清单，幂等。
    ///
    /// 已存在的行动保持原状态不变；超上限的候选落库为 `deferred`，
    /// 超过保留期的 `deferred` 转 `expired`。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn generate_day(
        &self,
        today: NaiveDate,
        daily_cap: u8,
    ) -> Result<Vec<Action>, DomainError> {
        let mut trace = OpTrace::begin("generate_day");
        {
            let mut connection = self.guard();
            let transaction = connection.transaction()?;

            let cutoff = encode_date(today - Duration::days(DEFERRED_RETENTION_DAYS));
            transaction.execute(
                "UPDATE actions SET status = 'expired'
                  WHERE status = 'deferred' AND on_date < ?1",
                params![cutoff],
            )?;

            let tasks = load_active_tasks(&transaction)?;
            let progress = load_all_progress(&transaction)?;
            let existing = load_actions(&transaction, today)?;

            let plan = plan_day(&PlanInput {
                today,
                daily_cap,
                tasks: &tasks,
                progress: &progress,
                existing: &existing,
            });

            let stamp = now_rfc3339();
            let on_date = encode_date(today);
            // seq 只能往后接续，否则每次刷新都会把新行动插到清单前部。
            let mut seq = i64::from(
                existing
                    .iter()
                    .map(|action| action.seq)
                    .max()
                    .map_or(0, |max| max.saturating_add(1)),
            );
            for planned in &plan.actions {
                let inserted = transaction.execute(
                    "INSERT OR IGNORE INTO actions
                        (task_id, on_date, kind, review_level, status, seq, created_at)
                     VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)",
                    params![
                        planned.task_id.0,
                        on_date,
                        planned.kind.as_str(),
                        planned.review_level,
                        seq,
                        stamp
                    ],
                )?;
                if inserted > 0 && planned.kind == ActionKind::Regular {
                    Self::touch_task_generated_on(&transaction, planned.task_id, today)?;
                }
                seq += 1;
            }

            for planned in &plan.deferred {
                transaction.execute(
                    "INSERT OR IGNORE INTO actions
                        (task_id, on_date, kind, review_level, status, seq, created_at)
                     VALUES (?1, ?2, ?3, ?4, 'deferred', ?5, ?6)",
                    params![
                        planned.task_id.0,
                        on_date,
                        planned.kind.as_str(),
                        planned.review_level,
                        seq,
                        stamp
                    ],
                )?;
                seq += 1;
            }

            transaction.commit()?;
        }
        let actions = self.actions_on(today)?;
        trace.ok();
        Ok(actions)
    }

    /// 某个任务在遗忘曲线上的位置。
    ///
    /// # Errors
    ///
    /// 任务不存在时返回 [`DomainError::NotFound`]。
    pub fn review_progress(&self, task: TaskId) -> Result<ReviewProgress, DomainError> {
        let mut trace = OpTrace::begin("review_progress");
        let connection = self.guard();
        let progress = connection
            .query_row(
                "SELECT task_id, level, next_review_on, graduated
                   FROM review_progress WHERE task_id = ?1",
                params![task.0],
                map_progress,
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "任务的复习进度",
                id: task.0,
            })?;
        trace.ok();
        Ok(progress)
    }

    /// 重置某个任务的复习进度。
    ///
    /// # Errors
    ///
    /// 任务不存在时返回 [`DomainError::NotFound`]。
    pub fn reset_review_progress(&self, task: TaskId) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("reset_review_progress");
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE review_progress
               SET level = 0, next_review_on = NULL, graduated = 0, updated_at = ?2
             WHERE task_id = ?1",
            params![task.0, now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "任务的复习进度",
                id: task.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 当天仍待完成的行动数量。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn pending_count(&self, today: NaiveDate) -> Result<usize, DomainError> {
        let mut trace = OpTrace::begin("pending_count");
        let connection = self.guard();
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM actions WHERE on_date = ?1 AND status = 'pending'",
            params![encode_date(today)],
            |row| row.get(0),
        )?;
        trace.ok();
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    /// 当天被顺延的行动数量。
    ///
    /// # Errors
    ///
    /// 读写失败时返回 [`DomainError::Store`]。
    pub fn deferred_count(&self, today: NaiveDate) -> Result<usize, DomainError> {
        let mut trace = OpTrace::begin("deferred_count");
        let connection = self.guard();
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM actions WHERE on_date = ?1 AND status = 'deferred'",
            params![encode_date(today)],
            |row| row.get(0),
        )?;
        trace.ok();
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    /// 结算一条行动：标记完成或跳过。
    ///
    /// 完成会推进遗忘曲线；跳过不推进。两种结算都会把变更前的复习进度快照
    /// 写入 `action_settlements`，供撤销回放。
    ///
    /// # Errors
    ///
    /// 行动不存在时返回 [`DomainError::NotFound`]；复习级别越界时返回
    /// [`DomainError::InvalidValue`]。
    pub fn settle_action(&self, action: ActionId, next: ActionStatus) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("settle_action");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;

        let current = load_action(&transaction, action)?;
        let mut progress = load_progress(&transaction, current.task_id)?;

        transaction.execute(
            "INSERT INTO action_settlements
                (action_id, prev_status, prev_level, prev_next_review_on, prev_graduated, settled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                action.0,
                current.status.as_str(),
                progress.level,
                progress.next_review_on.map(encode_date),
                i64::from(progress.graduated),
                now_rfc3339()
            ],
        )?;

        if next == ActionStatus::Done {
            let today = crate::store::decode_date(&encode_date(chrono::Local::now().date_naive()))
                .unwrap_or_else(|| chrono::Local::now().date_naive());
            progress =
                advance_review(&progress, today, &REVIEW_INTERVALS_DAYS).map_err(|error| {
                    DomainError::InvalidValue {
                        field: "复习级别",
                        expected: error.to_string(),
                    }
                })?;
            transaction.execute(
                "UPDATE review_progress
                    SET level = ?2, next_review_on = ?3, graduated = ?4, updated_at = ?5
                  WHERE task_id = ?1",
                params![
                    current.task_id.0,
                    progress.level,
                    progress.next_review_on.map(encode_date),
                    i64::from(progress.graduated),
                    now_rfc3339()
                ],
            )?;
        }

        transaction.execute(
            "UPDATE actions SET status = ?2, settled_at = ?3 WHERE id = ?1",
            params![action.0, next.as_str(), now_rfc3339()],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(())
    }

    /// 撤销某条行动最近一次结算。
    ///
    /// # Errors
    ///
    /// 该行动没有可撤销的结算时返回 [`DomainError::NotFound`]。
    pub fn undo_last_settlement(&self, action: ActionId) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("undo_last_settlement");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;

        let snapshot: Option<(i64, String, i64, Option<String>, i64)> = transaction
            .query_row(
                "SELECT id, prev_status, prev_level, prev_next_review_on, prev_graduated
                   FROM action_settlements
                  WHERE action_id = ?1
                  ORDER BY id DESC LIMIT 1",
                params![action.0],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;

        let Some((settlement_id, prev_status, prev_level, prev_next_review_on, prev_graduated)) =
            snapshot
        else {
            return Err(DomainError::NotFound {
                entity: "可撤销的结算记录",
                id: action.0,
            });
        };

        let task_id = load_action(&transaction, action)?.task_id;
        transaction.execute(
            "UPDATE review_progress
                SET level = ?2, next_review_on = ?3, graduated = ?4, updated_at = ?5
              WHERE task_id = ?1",
            params![
                task_id.0,
                prev_level,
                prev_next_review_on,
                prev_graduated,
                now_rfc3339()
            ],
        )?;
        transaction.execute(
            "UPDATE actions SET status = ?2, settled_at = NULL WHERE id = ?1",
            params![action.0, prev_status],
        )?;
        transaction.execute(
            "DELETE FROM action_settlements WHERE id = ?1",
            params![settlement_id],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(())
    }
}

fn load_actions(connection: &Connection, today: NaiveDate) -> Result<Vec<Action>, DomainError> {
    let mut statement = connection.prepare(
        "SELECT id, task_id, on_date, kind, review_level, status, seq, note
           FROM actions WHERE on_date = ?1 ORDER BY seq, id",
    )?;
    let rows = statement.query_map(params![encode_date(today)], map_action)?;
    let mut actions = Vec::new();
    for row in rows {
        actions.push(row?);
    }
    Ok(actions)
}

fn load_action(connection: &Connection, action: ActionId) -> Result<Action, DomainError> {
    connection
        .query_row(
            "SELECT id, task_id, on_date, kind, review_level, status, seq, note
               FROM actions WHERE id = ?1",
            params![action.0],
            map_action,
        )
        .optional()?
        .ok_or(DomainError::NotFound {
            entity: "行动",
            id: action.0,
        })
}

fn load_active_tasks(connection: &Connection) -> Result<Vec<Task>, DomainError> {
    let mut statement = connection.prepare(
        "SELECT id, category_id, name, freq_kind, freq_interval, estimated_minutes,
                status, last_generated_on, created_at, description
           FROM tasks WHERE deleted_at IS NULL AND status = 'active' ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        let kind: String = row.get(3)?;
        let interval: i64 = row.get(4)?;
        let status: String = row.get(6)?;
        let last_generated: Option<String> = row.get(7)?;
        let description: Option<String> = row.get(9)?;
        Ok(Task {
            id: TaskId(row.get(0)?),
            category_id: crate::model::CategoryId(row.get(1)?),
            name: row.get(2)?,
            frequency: crate::model::Frequency::from_parts(&kind, interval)
                .unwrap_or(crate::model::Frequency::Daily),
            estimated_minutes: row.get(5)?,
            description,
            status: TaskStatus::parse(&status),
            last_generated_on: last_generated.as_deref().and_then(decode_date),
            created_at: row.get(8)?,
        })
    })?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row?);
    }
    Ok(tasks)
}

fn load_all_progress(connection: &Connection) -> Result<Vec<ReviewProgress>, DomainError> {
    let mut statement = connection.prepare(
        "SELECT task_id, level, next_review_on, graduated FROM review_progress ORDER BY task_id",
    )?;
    let rows = statement.query_map([], map_progress)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

fn load_progress(connection: &Connection, task: TaskId) -> Result<ReviewProgress, DomainError> {
    connection
        .query_row(
            "SELECT task_id, level, next_review_on, graduated
               FROM review_progress WHERE task_id = ?1",
            params![task.0],
            map_progress,
        )
        .optional()?
        .ok_or(DomainError::NotFound {
            entity: "任务的复习进度",
            id: task.0,
        })
}

fn map_action(row: &rusqlite::Row<'_>) -> rusqlite::Result<Action> {
    let on_date: String = row.get(2)?;
    let kind: String = row.get(3)?;
    let status: String = row.get(5)?;
    Ok(Action {
        id: ActionId(row.get(0)?),
        task_id: TaskId(row.get(1)?),
        on_date: decode_date(&on_date).unwrap_or_else(|| chrono::Local::now().date_naive()),
        kind: ActionKind::parse(&kind),
        review_level: row.get(4)?,
        status: ActionStatus::parse(&status),
        seq: row.get(6)?,
        note: row.get(7)?,
    })
}

pub(crate) fn map_progress(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReviewProgress> {
    let next_review_on: Option<String> = row.get(2)?;
    let graduated: i64 = row.get(3)?;
    Ok(ReviewProgress {
        task_id: TaskId(row.get(0)?),
        level: row.get(1)?,
        next_review_on: next_review_on.as_deref().and_then(decode_date),
        graduated: graduated != 0,
    })
}
