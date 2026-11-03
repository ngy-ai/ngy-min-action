# Phase 1 Data Model: 名称唯一性与规范化

**Feature**: `002-name-uniqueness` | **Date**: 2026-11-03 | **Store**: SQLite（单文件）

**基线**: 在 `001-min-action-core` 的表结构上做增量，只改 `goals` 与 `categories` 两张表，其余表结构不变。

## 1. goals — 目标

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `name` | TEXT | NOT NULL, 1..=80 字符（去两端空白后非空），用于展示 | FR-001 / FR-006 |
| `name_folded` | TEXT | NOT NULL, 由 `name` 折叠得到，用于比较 | FR-003 |
| `description` | TEXT | NULL, <= 500 字符 | 001 FR-001 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | 001 FR-032 |
| `deleted_at` | TEXT | NULL | 001 FR-005 |

**唯一约束**: `CREATE UNIQUE INDEX idx_goals_name_folded ON goals(name_folded)` —— 整表唯一，不带 `WHERE deleted_at IS NULL`。

**不变量**:

- `name_folded` 必须等于 `name` 的 Unicode 默认大小写折叠结果，任何写入路径都要同时写两列
- 已软删除的目标仍占用名称（FR-001）
- `name` 内部空白原样保留，只有两端被去除（FR-006 / FR-007）

## 2. categories — 分类

| 列 | 类型 | 约束 | 来源 |
| --- | --- | --- | --- |
| `id` | INTEGER | PK AUTOINCREMENT | — |
| `goal_id` | INTEGER | NOT NULL, FK → goals(id) ON DELETE CASCADE | 001 FR-002 |
| `name` | TEXT | NOT NULL, 1..=80 字符（去两端空白后非空），用于展示 | FR-002 / FR-006 |
| `name_folded` | TEXT | NOT NULL, 由 `name` 折叠得到，用于比较 | FR-003 |
| `created_at` | TEXT | NOT NULL, RFC 3339 | — |
| `deleted_at` | TEXT | NULL | — |

**唯一约束**: `CREATE UNIQUE INDEX idx_categories_goal_name_folded ON categories(goal_id, name_folded)` —— 整表唯一，不带 `WHERE`。

**不变量**:

- 同一目标下名称唯一（含已删除分类）；不同目标之间允许同名（FR-002）
- 已删除的分类仍占用所属目标下的名称

**索引迁移**: 旧索引 `idx_categories_live_name`（部分索引，`WHERE deleted_at IS NULL`）在迁移中被整体删除，不保留。

## 3. 架构版本与迁移

**当前版本**: 1 → **目标版本**: 2（`src/store/schema.rs::SCHEMA_VERSION`）

| 起点状态 | 动作 |
| --- | --- |
| 库不存在或 `user_version = 0` 且无表 | 建表、写 `user_version = 2` |
| `0 < user_version < 2` | 事务内 `DROP` 全部表 → 建表 → 写 `user_version = 2` |
| `user_version = 2` | 不做任何事 |

**`DROP` 顺序**（`foreign_keys=ON` 时不得颠倒）:

```text
action_settlements → actions → review_progress → tasks → categories → goals → reminder_state → settings
```

**失败语义**:

- 任一步失败整体回滚，`user_version` 保持原值
- 程序以 `StoreError::Migrate` 退出，不在半完成状态上继续运行
- 下次启动重新尝试迁移，不会出现「已清空但未建表」的中间态

## 4. 校验与规范化管线

```text
用户输入
  └─ validate_name(raw, field, max)
       ├─ trim 两端空白            → 空则 InvalidName::Empty
       ├─ 长度与控制字符校验        → InvalidName::TooLong / ControlChar
       └─ 返回展示名（已 trim）
            └─ fold_name(展示名)   → name_folded
                 └─ 唯一索引判重    → DuplicateName { scope, name }
```

**比较口径**（FR-003）: 去两端空白 → Unicode 默认大小写折叠 → 二进制比较。折叠后相同即视为同一名称。

**不折叠的内容**: 名称内部的空白、全角与半角差异、繁简差异。这些都由用户自行区分（见规格 Assumptions）。

## 5. 受影响的其他实体

无。`tasks`、`actions`、`review_progress`、`action_settlements`、`reminder_state`、`settings` 的表结构与约束不变，只在迁移时随整库清空一并重建。
