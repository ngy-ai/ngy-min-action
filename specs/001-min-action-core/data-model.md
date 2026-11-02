# Phase 1 Data Model: 最小行动管理系统

**Feature**: `001-min-action-core` | **Date**: 2026-11-02 | **Store**: SQLite（单文件）

## 实体总览

```text
goals 1──n categories 1──n tasks 1──n actions
                                  └─1 review_progress
reminder_state (单行)   settings (键值)
```

## 1. goals — 目标

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `name` | TEXT | NOT NULL, 1..=80 字符（去空白后非空） | FR-001 / FR-004 / FR-033 |
| `description` | TEXT | NULL, <= 500 字符 | FR-001 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | FR-032 |

**Validation**: 名称去首尾空白后非空；拒绝含换行、制表符与其他控制字符的输入（FR-033 / FR-004）。

**删除语义**: 删除目标 → 级联删除其分类、任务、未完成行动与复习进度；`actions.status = 'done'` 的记录保留（FR-005），通过 `task_id` 置空或历史表二者取其一，见「删除与级联」。

## 2. categories — 分类

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `goal_id` | INTEGER | NOT NULL, FK → goals(id) ON DELETE CASCADE | FR-002 |
| `name` | TEXT | NOT NULL, 1..=80 字符 | FR-002 / FR-004 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | — |

**约束**: `UNIQUE(goal_id, name)` —— 同一目标内名称唯一，跨目标可重名（FR-002）。

## 3. tasks — 任务

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `category_id` | INTEGER | NOT NULL, FK → categories(id) ON DELETE CASCADE | FR-006 |
| `name` | TEXT | NOT NULL, 1..=120 字符 | FR-006 / FR-033 |
| `freq_kind` | TEXT | NOT NULL, `daily` \| `every_n_days` \| `weekly_n` | FR-007 |
| `freq_interval` | INTEGER | NOT NULL, 1..=365；`daily` 恒为 1 | FR-007 |
| `estimated_minutes` | INTEGER | NULL, 1..=480 | FR-006 |
| `status` | TEXT | NOT NULL, `active` \| `paused`，默认 `active` | FR-009 |
| `last_generated_on` | TEXT | NULL, `YYYY-MM-DD` | FR-008 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | — |

**频率判定**（FR-008，纯函数，`schedule::frequency`）：

- `daily`：`today > last_generated_on`（或 `last_generated_on` 为空）即生成
- `every_n_days`：`last_generated_on` 为空，或 `today - last_generated_on >= n`
- `weekly_n`：等价于 `every_n_days`，间隔取 `⌊7 / n⌋`（`n = 7` 时退化为 `daily`）

**状态转移**: `active ⇄ paused`。转入 `paused` 时清空当天候选资格；恢复后 `last_generated_on` 保持原值，因此暂停期间的自然日不补生成（FR-009）。

## 4. actions — 行动

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `task_id` | INTEGER | NOT NULL, FK → tasks(id) | FR-010 |
| `on_date` | TEXT | NOT NULL, `YYYY-MM-DD` | FR-010 |
| `kind` | TEXT | NOT NULL, `regular` \| `review` | FR-010 |
| `review_level` | INTEGER | NULL；`kind = 'review'` 时非空，1..=6 | FR-010 / FR-019 |
| `status` | TEXT | NOT NULL, `pending` \| `done` \| `skipped` \| `deferred` \| `expired` | FR-013 / FR-017 |
| `seq` | INTEGER | NOT NULL，当日排序位次，用于稳定展示 | FR-014 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | — |
| `settled_at` | TEXT | NULL，状态离开 `pending` 的时间 | FR-017 |

**约束**: `UNIQUE(task_id, on_date)` —— 这是 FR-011 幂等性的数据库级保证：同一天同一任务最多一行，重复生成走 `INSERT ... ON CONFLICT DO NOTHING`。

**状态转移**:

```text
pending ──完成──> done        （FR-017，推进复习）
pending ──跳过──> skipped     （FR-017，次日重新出现）
pending ──超上限──> deferred  （FR-013，次日重新参与竞争）
deferred ──次日入选──> pending
deferred ──保留超过 7 天──> expired（FR-015，避免积压无限增长）
done ──撤销最近一次标记──> pending（FR-017）
```

**顺延的表示**: `deferred` 行动不预先分配未来日期，而是在之后每天重新进入候选池参与竞争。这样「顺延一条」不会连带重排后续所有计划，也天然满足 FR-015（连续多日未使用时不倾倒积压）。

**过期策略**: `deferred` 超过 7 天转 `expired`，同时写一条 `warn` 日志。理由见 `plan.md` 复杂度追踪。

## 5. review_progress — 复习进度

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `task_id` | INTEGER | PK, FK → tasks(id) ON DELETE CASCADE | FR-018 |
| `level` | INTEGER | NOT NULL, 0..=6，0 表示未开始 | FR-021 |
| `next_review_on` | TEXT | NULL, `YYYY-MM-DD` | FR-018 / FR-021 |
| `graduated` | INTEGER | NOT NULL, 0 \| 1，默认 0 | FR-023 |
| `updated_at` | TEXT | NOT NULL, RFC 3339 | — |

**遗忘曲线序列**（单一常量，`schedule::review::INTERVALS_DAYS`）：`[1, 2, 4, 7, 15, 30]`。

- 完成任务后：`level = 1`，`next_review_on = 完成日 + 1 天`
- 完成第 k 级复习后：`level = k + 1`，`next_review_on = 完成日 + INTERVALS_DAYS[k]`
- `level = 6` 完成后：`graduated = 1`，`next_review_on = NULL`（FR-023）
- 重置：`level = 0`、`next_review_on = NULL`、`graduated = 0`（FR-022）
- 撤销标记：把 `level`、`next_review_on`、`graduated` 恢复到标记前的快照（FR-017 的「撤销最近一次标记」）

**撤销的实现约束**: `actions.settled_at` 之外，需要用一张 `action_settlements` 审计表保存每次状态变更前的复习快照，撤销时回放最近一条。该表只增不改，也顺带满足「已完成历史保留」（FR-005）。

## 6. reminder_state — 提示记录（单行）

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK, `CHECK (id = 1)` | FR-031 |
| `last_notified_on` | TEXT | NULL, `YYYY-MM-DD` | FR-026 / FR-029 |
| `last_viewed_on` | TEXT | NULL, `YYYY-MM-DD` | FR-027 |

**判定**（`notify::should_notify`）：`last_notified_on != today` 且 `last_viewed_on != today` 且当天存在 `pending` 行动（FR-028）。这三条同时成立才弹窗，弹窗后立刻写回 `last_notified_on = today`，从而在程序重启后仍然只弹一次（FR-031）。

## 7. settings — 键值配置

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `key` | TEXT | PK | FR-012 |
| `value` | TEXT | NOT NULL | FR-012 |

**本期只需一个键**：`daily_cap`（正整数，默认 `5`，范围 1..=20）。每任务上限固定为 1，作为常量而非配置项（FR-012 明确本版不提供调整）。

## 校验规则汇总（边界校验，宪法 VIII）

| 输入 | 规则 | 失败提示要点 |
| --- | --- | --- |
| 目标名称 | 去空白后 1..=80 字符，无控制字符 | 指出是名称、允许长度、如何修正 |
| 分类名称 | 去空白后 1..=80 字符，同目标内唯一 | 指出重复的是哪个分类 |
| 任务名称 | 去空白后 1..=120 字符，无控制字符 | 同上 |
| 频率 | 合法枚举；`every_n_days` 的 n ∈ 1..=365 | 列出可用取值 |
| 预计耗时 | 可空；非空时 1..=480 分钟 | 指出单位与范围 |
| 每日上限 | 1..=20 | 指出默认值与允许范围 |

所有校验在写入数据库之前完成，校验失败不产生任何行（FR-004 / FR-034）。
