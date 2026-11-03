# Phase 1 Data Model: 任务频率选择与复习进度展示

**Feature**: `003-task-form-and-progress` | **Date**: 2026-11-03

## 表结构

不变。频率已存在于 `tasks.freq_kind` / `freq_interval`，复习进度已存在于 `review_progress`。本 feature 不新增列、不新增索引、不改架构版本。

## 新增读取路径

`Store::progress_for_category(category)` —— 一次查询返回某分类下全部未删除任务的复习进度。

| 输入 | 输出 | 说明 |
| --- | --- | --- |
| `CategoryId` | `Vec<ReviewProgress>` | 只含该分类下未删除任务；无进度的任务不出现在结果中 |

**SQL**:

```sql
SELECT rp.task_id, rp.level, rp.next_review_on, rp.graduated
  FROM review_progress rp
  JOIN tasks t ON t.id = rp.task_id
 WHERE t.category_id = ?1 AND t.deleted_at IS NULL
 ORDER BY rp.task_id
```

**为什么用 JOIN 而不是逐个 `review_progress(task)`**: 任务列表一次渲染需要全部进度，逐个查询会产生 N+1。

## 展示映射

| 数据 | 展示 |
| --- | --- |
| `Frequency::Daily` | 每天 |
| `Frequency::EveryNDays(n)` | 每 N 天 |
| `Frequency::WeeklyN(n)` | 每周 N 次 |
| 无进度记录 | 未开始 |
| `level = 1..=5` | 第 N 级，下次复习 YYYY-MM-DD |
| `graduated != 0` | 已巩固（不显示下次复习日） |

## 写入路径

不变。`create_task` 已经写入 `draft.frequency.kind()` 与 `raw_interval()`，本 feature 只是让界面提供非默认的 `frequency`。
