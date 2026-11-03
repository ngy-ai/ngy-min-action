# Phase 0 Research: 名称唯一性与规范化

**Feature**: `002-name-uniqueness` | **Date**: 2026-11-03

## D1. Unicode 大小写折叠的实现

**问题**: FR-003 要求比较覆盖全部 Unicode 大小写折叠，而 SQLite 与 Rust 标准库各自都不完整。

**选项**:

| 方案 | 覆盖范围 | 代价 |
| --- | --- | --- |
| `str::to_lowercase` | 简单小写映射。`ß` 折叠后仍是 `ß`（正确折叠应为 `ss`），希腊语词尾 `ς` 不归一到 `σ` | 零依赖，但达不到「全部」 |
| `COLLATE NOCASE` | 仅 ASCII 的 A–Z | 零代码，SQLite 文档明确限制 |
| `caseless` 0.2 | 按 Unicode 5.2 的 CaseFolding.txt 做默认折叠与规范化比较 | 新增一个纯 Rust 依赖 |

**决策**: 引入 `caseless = "0.2"`，用其默认折叠函数产出比较键。

**理由**: 「覆盖全部 Unicode」是本 feature 的显式需求，前两个方案都会在实测中留下 `ß` / `ς` 一类反例。`caseless` 无 unsafe、无额外传递依赖，只为一个纯函数引入，复杂度可接受。这也符合宪法 VI——依赖编码了标准库不具备的不变量。

**风险**: 折叠函数的确切签名需在实现第一步核对（`cargo doc -p caseless`），若 API 与预期不符，退回到「先 `to_lowercase` 再 `NFKC` 规范化」的组合并在本文件记录原因。

## D2. 唯一键的落地方式

**问题**: 折叠后的比较要参与唯一约束，且约束必须覆盖已删除记录。

**选项**:

| 方案 | 说明 | 问题 |
| --- | --- | --- |
| 在 `name` 上建 `COLLATE NOCASE` 唯一索引 | 改动最小 | 折叠能力只到 ASCII，与 D1 冲突 |
| 新增 `name_folded` 列，在其上建唯一索引 | 写入时同时落展示名与折叠名，索引做二进制比较 | 多一列、多一处写入点 |
| 每次写入前全表扫描比对 | 不落库也能判重 | 与宪法 V 的「对可索引内容做线性查找」冲突 |

**决策**: 新增 `name_folded` 列。

- `goals`: `CREATE UNIQUE INDEX idx_goals_name_folded ON goals(name_folded)`（整表，无 `WHERE`）
- `categories`: `CREATE UNIQUE INDEX idx_categories_goal_name_folded ON categories(goal_id, name_folded)`（整表，无 `WHERE`）

**理由**: 索引承担判重，应用层只负责在写入前算出折叠值。唯一性覆盖已删除行靠「不带部分索引条件」直接表达，不需要额外的触发器或应用层补查。

## D3. 迁移策略

**问题**: 旧库已有重名目标，且 `CREATE TABLE IF NOT EXISTS` 不会补上 `name_folded` 列。

**选项**:

| 方案 | 说明 | 问题 |
| --- | --- | --- |
| `ALTER TABLE ADD COLUMN` + 清洗重名行 | 保留数据 | 与 Q1 决策「清空全部业务数据」矛盾，清洗逻辑白写 |
| `DROP` 全部表后按新 DDL 重建 | 数据全清，结构干净 | 不可逆；必须整体事务 |
| 拒绝打开并提示用户手工删库 | 最省代码 | 表现为启动失败，规格 FR-008 明确不允许 |

**决策**: `DROP` + 重建，整体事务。

**步骤**（`src/store/schema.rs::apply`）:

1. 读 `PRAGMA user_version`
2. `user_version == 0` 且无表 → 新库，直接建表并写版本
3. `0 < user_version < SCHEMA_VERSION` → 事务内按 `action_settlements → actions → review_progress → tasks → categories → goals → reminder_state → settings` 顺序 `DROP TABLE IF EXISTS`，再执行新 DDL，最后写 `user_version`
4. `user_version == SCHEMA_VERSION` → 不做任何事

**理由**: 既然数据本就要清空，`DROP` 比「`ALTER` + 清洗 + 逐表删行」少一半步骤，也不会留下旧列的默认值问题。`foreign_keys=ON` 下必须按子表到父表的顺序 `DROP`，因为 `PRAGMA foreign_keys` 在事务内无法切换。

**已知限制**: 迁移不可逆，且不提供备份。规格 Assumptions 已声明用户需自行备份数据库文件。

## D4. 错误文案的形态

**问题**: `DomainError::DuplicateName { scope, name }` 当前拼成「{scope} 下已存在名称「{name}」」，分类传 `scope = "同一目标下的分类"`。目标没有上位作用域，套用会得到「目标 下已存在名称」。

**选项**:

| 方案 | 说明 | 问题 |
| --- | --- | --- |
| 新增 `DuplicateGoalName { name }` 变体 | 文案可完全定制 | 两个变体表达同一语义，接触面分裂 |
| `scope` 改为完整句子，模板改为「{scope}：已存在名称「{name}」，请换一个名称或先删除原有的那一项」 | 一个变体覆盖两种作用域 | 改动既有文案，需在迁移说明中记录 |

**决策**: 后者。

- 目标：`scope = "目标名称必须唯一"`
- 分类：`scope = "同一目标下的分类名称必须唯一"`

**理由**: 宪法 IV 要求错误可执行且结构统一，一个变体两种 scope 比两个语义相同的变体更不容易用错。文案改动影响既有测试与界面提示，按接触面变更在 `plan.md` 的复杂度追踪中登记。
