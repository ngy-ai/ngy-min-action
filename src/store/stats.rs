//! 复习统计：窗口分项、连续达成天数与阶段分布。

use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, params};

use crate::model::{ActionTally, DomainError, ReviewStats};
use crate::store::{OpTrace, Store, encode_date};

impl Store {
    /// 汇总复习统计。
    ///
    /// `window_days` 是完成率的统计窗口（含今天）；连续天数按全量历史计算。
    /// 行动统计不排除已软删除的任务——过期行动只可能由删除任务产生，
    /// 把那批行动排除会让过期数恒为零。
    ///
    /// # Errors
    ///
    /// 聚合查询失败时返回 [`DomainError::Store`]。
    pub fn review_stats(
        &self,
        today: NaiveDate,
        window_days: u32,
    ) -> Result<ReviewStats, DomainError> {
        let mut trace = OpTrace::begin("review_stats");
        let connection = self.guard();
        let tally = tally_in_window(&connection, today, window_days)?;
        let (current_streak, longest_streak) = streaks(&connection, today)?;
        let (untouched_tasks, learning_tasks, graduated_tasks) = stage_counts(&connection)?;
        trace.ok();
        Ok(ReviewStats {
            window_days,
            tally,
            current_streak,
            longest_streak,
            untouched_tasks,
            learning_tasks,
            graduated_tasks,
        })
    }
}

/// 窗口内按状态分项计数。
fn tally_in_window(
    connection: &Connection,
    today: NaiveDate,
    window_days: u32,
) -> Result<ActionTally, DomainError> {
    let span = i64::from(window_days.max(1)) - 1;
    let start = today - Duration::days(span);
    let (done, skipped, expired) = connection.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN status = 'expired' THEN 1 ELSE 0 END), 0)
          FROM actions
          WHERE on_date >= ?1 AND on_date <= ?2",
        params![encode_date(start), encode_date(today)],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    Ok(ActionTally {
        done: usize::try_from(done).unwrap_or_default(),
        skipped: usize::try_from(skipped).unwrap_or_default(),
        expired: usize::try_from(expired).unwrap_or_default(),
    })
}

/// 当前连续达成天数与历史最长连续达成天数。
///
/// 倒序遍历「有行动的日子」：当天有完成即达成；当天零完成且早于今天则中断；
/// 当天零完成但就是今天则跳过——今天还没结束。没有行动的日子不出现在聚合里，
/// 自然不打断连续。
fn streaks(connection: &Connection, today: NaiveDate) -> Result<(u32, u32), DomainError> {
    let mut statement = connection.prepare(
        "SELECT on_date, SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) AS done_count
           FROM actions
          GROUP BY on_date
          ORDER BY on_date DESC",
    )?;
    let mut rows = statement.query([])?;
    let today_raw = encode_date(today);

    let mut current = 0_u32;
    let mut longest = 0_u32;
    let mut running = 0_u32;
    let mut current_open = true;

    while let Some(row) = rows.next()? {
        let day: String = row.get(0)?;
        let done: i64 = row.get(1)?;
        if done > 0 {
            running += 1;
            if current_open {
                current = running;
            }
            longest = longest.max(running);
        } else if day == today_raw {
            continue;
        } else {
            current_open = false;
            running = 0;
        }
    }
    Ok((current, longest))
}

/// 当前任务库按复习阶段的分组计数：未开始、学习中、已巩固。
fn stage_counts(connection: &Connection) -> Result<(usize, usize, usize), DomainError> {
    let (untouched, learning, graduated) = connection.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN p.level = 0 AND p.graduated = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN p.level > 0 AND p.graduated = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN p.graduated = 1 THEN 1 ELSE 0 END), 0)
          FROM review_progress p
          JOIN tasks t ON t.id = p.task_id
          WHERE t.deleted_at IS NULL",
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    Ok((
        usize::try_from(untouched).unwrap_or_default(),
        usize::try_from(learning).unwrap_or_default(),
        usize::try_from(graduated).unwrap_or_default(),
    ))
}
