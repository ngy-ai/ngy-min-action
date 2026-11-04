# Tasks: 目标与分类名称的唯一性与规范化

**Input**: Design documents from `/specs/002-name-uniqueness/`

**Prerequisites**: [plan.md](./plan.md)、[spec.md](./spec.md)、[research.md](./research.md)、[data-model.md](./data-model.md)、[contracts/domain-api.md](./contracts/domain-api.md)

**Tests**: 本 feature 强制测试先行（宪法 III），每个用户故事都含必须先失败的测试任务。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 不同文件且无未完成依赖时才可并行；同文件的多个任务一律不并行
- **[Story]**: 对应 spec.md 的用户故事

## Path Conventions

单 crate：源码在 `src/`，集成测试在 `tests/`，测试文件后缀 `_test.rs`。

---

## Phase 1: Setup

**Purpose**: 引入折叠依赖并把折叠能力落成语义明确的函数

- [X] T001 在 `Cargo.toml` 的 `[dependencies]` 加入 `caseless = "0.2"`，运行 `cargo doc -p caseless --no-deps` 核对默认折叠函数的确切签名；若与 `research.md` D1 的预期不符，先在该文件记录原因并确定替代函数
- [X] T002 在 `src/model.rs` 的 `#[cfg(test)] mod tests` 写入 `fold_name` 用例：`ß → ss`、`ΣΟΦΟΣ` 与 `σοφος` 折叠后相等、中文与数字保持原样、对已折叠结果再次折叠不变（幂等）
- [X] T003 在 `src/model.rs` 实现 `pub fn fold_name(name: &str) -> String`，让 T002 转绿；契约注释写明「输入须已通过 `validate_name`，纯函数且幂等」

**Checkpoint**: 折叠函数可用于后续所有写入路径

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 唯一键的持久化结构，任何用户故事都依赖它

**⚠️ CRITICAL**: 本阶段完成前，任何用户故事都无法独立验收

- [X] T004 [P] 在 `src/store/schema.rs` 把 `SCHEMA_VERSION` 改为 2；`goals` 与 `categories` 增加 `name_folded TEXT NOT NULL`；删除 `idx_categories_live_name`，新增 `idx_goals_name_folded UNIQUE(goals.name_folded)` 与 `idx_categories_goal_name_folded UNIQUE(categories.goal_id, categories.name_folded)`（两者都不带 `WHERE deleted_at IS NULL`）
- [X] T005 在 `src/store/schema.rs` 把 `apply` 改为按 `PRAGMA user_version` 分支：`0` → 建表并写版本；`== 2` → 直接返回；其余版本暂时返回 `Migrate` 错误（由 T018 补齐）
- [X] T006 [P] 在 `src/store/goal.rs` 的四个写入语句中补 `name_folded` 列（值来自 `fold_name(&name)`），保证 DDL 的 `NOT NULL` 不被插入路径打破

**Checkpoint**: 新库可直接建出带唯一索引的结构，四个写入路径不再违反约束

---

## Phase 3: User Story 1 - 目标名称唯一 (Priority: P1) 🎯 MVP

**Goal**: 创建或重命名目标时，折叠后同名的请求被拒绝，原数据不变

**Independent Test**: 从空库创建「学习英语」后再提交「学习英语」「learning english」「  学习英语  」，全部被拒且列表仍一条；把另一个目标重命名为「学习英语」被拒。

### Tests for User Story 1 ⚠️

> 先写测试并运行到失败，再写实现（宪法 III）

- [X] T007 [US1] 在 `tests/store_test.rs` 追加用例：重名目标创建被拒且不落行、仅大小写不同的目标被拒（`Learning English` / `learning english`）、`straße` 与 `STRASSE` 被视为重名、`ΣΟΦΟΣ` 与 `σοφος` 被视为重名、重命名到已存在名称被拒、删除目标后同名不可重建

### Implementation for User Story 1

- [X] T008 [US1] 在 `src/store/goal.rs` 的 `create_goal` 把唯一冲突翻译为 `DuplicateName { scope: "目标名称必须唯一", name }`，错误路径不得留下任何行
- [X] T009 [US1] 在 `src/store/goal.rs` 的 `rename_goal` 同时更新 `name_folded`，并把唯一冲突翻译为同一 `scope`；与自身同名不算冲突
- [X] T010 [US1] 在 `src/model.rs` 把 `DomainError::DuplicateName` 的展示模板改为「{scope}：已存在名称「{name}」，请换一个名称或先删除原有的那一项」
- [X] T011 [US1] 在 `src/store/goal.rs` 把 `rename_category` 的 `scope` 改为「同一目标下的分类名称必须唯一」，并同步调整既有断言中依赖旧文案的用例

**Checkpoint**: 目标的创建与重命名具备唯一性约束，MVP 可验收

---

## Phase 4: User Story 2 - 分类名称在同一目标下唯一 (Priority: P1)

**Goal**: 同一目标下分类重名被拒，跨目标可重名；重命名同样受约束

**Independent Test**: 一个目标下建「口语」后重复提交被拒，另一个目标下建「口语」成功；把同目标下的「语法」改名为「口语」被拒。

### Tests for User Story 2 ⚠️

- [X] T012 [US2] 在 `tests/store_test.rs` 追加用例：同一目标下重名分类被拒、仅大小写不同的分类名被拒、跨目标同名分类允许、分类重命名冲突被拒、删除分类后同目标下不可重建；并把既有 `soft_deleted_category_disappears_from_lists` 中「删除后应允许重建同名分类」改为反向断言

### Implementation for User Story 2

- [X] T013 [US2] 在 `src/store/goal.rs` 的 `create_category` 翻译唯一冲突为 `DuplicateName { scope: "同一目标下的分类名称必须唯一", name }`
- [X] T014 [US2] 在 `src/store/goal.rs` 的 `rename_category` 更新 `name_folded` 并沿用上述 `scope`（T011 已改文案）

**Checkpoint**: US1 与 US2 均独立可用，作用域边界（全局 / 目标内）可分别验证

---

## Phase 5: User Story 3 - 名称两端空白被去除 (Priority: P2)

**Goal**: 入库与回显都是去掉两端空白后的名称，内部空白保留

**Independent Test**: 输入「  口语  」创建后列表显示「口语」；输入纯空格被拒；输入「学习 英语」与「学习英语」并存为两条。

### Tests for User Story 3 ⚠️

- [X] T015 [P] [US3] 在 `tests/store_test.rs` 追加用例：两端空白被去除后入库、纯空白被拒且不落行、内部空白原样保留（「学习 英语」与「学习英语」不判重）

### Implementation for User Story 3

- [X] T016 [US3] 核对 `src/model.rs::validate_name` 现有的 `trim` 行为是否已满足 T015；若已满足，仅保留测试并在任务说明中记录「既有实现已符合，本次为回归护栏」；若不满足才修改实现（宪法 III 只对新增生产代码要求先失败）

**Checkpoint**: 空白噪音不再绕过唯一性判定

---

## Phase 6: User Story 4 - 旧数据升级后可正常使用 (Priority: P3)

**Goal**: 旧版本数据库升级时清空全部业务数据，程序正常启动，且只清空一次

**Independent Test**: 用 `user_version = 1` 且含重名目标的库启动，数据为空且可新建；再次启动数据保留。

### Tests for User Story 4 ⚠️

- [X] T017 [US4] 在 `tests/store_test.rs` 追加迁移用例：用 `Store::open` 建库并写入目标 → drop → 用 `rusqlite::Connection` 把 `user_version` 设为 1 → 重新 `Store::open`，断言目标、分类、任务、行动全为空且可正常写入；再次 `Store::open` 断言数据保留（不重复清空）

### Implementation for User Story 4

- [X] T018 [US4] 在 `src/store/schema.rs` 补齐 T005 留下的迁移分支：`0 < user_version < 2` 时在单个事务内按 `action_settlements → actions → review_progress → tasks → categories → goals → reminder_state → settings` 顺序 `DROP TABLE IF EXISTS`，再执行 DDL，最后写 `user_version = 2`；失败整体回滚
- [X] T019 [US4] 在 `src/store/schema.rs` 的迁移路径记录一条结构化日志：`op = "store_migrate"`、`from`、`to`、`result`、`elapsed_ms`
- [X] T020 [US4] 在 `src/store.rs` 确认升级失败的映射仍为 `StoreError::Migrate` 且不改变原库内容，必要时补充错误上下文

**Checkpoint**: 升级路径不再表现为启动失败，且清空只发生一次

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T021 核对 [contracts/domain-api.md](./contracts/domain-api.md) 与实际实现是否一致（函数签名、文案、错误 `scope` 取值）；核对结论为一致，未做改动
- [X] T022 按 [quickstart.md](./quickstart.md) 的六个场景做手工验收，重点确认 `ß` / `ΣΟΦΟΣ` 两条折叠反例
- [X] T023 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`，三条全绿后提交

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖
- **Foundational (Phase 2)**: 依赖 Setup；阻塞全部用户故事
- **US1 / US2 / US3 / US4**: 均依赖 Phase 2
- **Polish (Phase 7)**: 依赖四个故事完成

### User Story Dependencies

- **US1 (P1)**: Phase 2 之后可开始，不依赖其他故事
- **US2 (P1)**: Phase 2 之后可开始；与 US1 共用 `src/store/goal.rs`，实际按 US1 → US2 顺序执行
- **US3 (P2)**: 只依赖 `validate_name`，可与 US1/US2 并行推进，但测试都在 `tests/store_test.rs`
- **US4 (P3)**: 依赖 T004/T005 的结构变更，与 US1/US2 无代码交集

### Within Each User Story

- 测试任务必须先跑出失败，再进入实现任务
- `tests/store_test.rs` 是单一文件，四个故事的测试任务不得并行
- 故事完成后再进入下一个优先级

### Parallel Opportunities

- T004 与 T006 分属 `store/schema.rs` 与 `store/goal.rs`，可并行
- T002（`model.rs` 测试）与 T004 可并行
- US4 的实现（T018–T020）与 US1/US2 的实现无文件交集，可穿插进行
- 其余任务受同文件约束，串行执行

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 → Phase 2
2. Phase 3（US1）→ 单独验收目标唯一性
3. 此时分类仍无折叠判重，不可交付

### Incremental Delivery

1. Setup + Foundational → 结构就绪
2. US1 + US2 → 作用域唯一性完整，可交付（这是本 feature 的最小可用集）
3. US3 → 空白规范化
4. US4 → 升级路径，缺它则旧库无法启动，不可发布

> US1 与 US2 合并才是 MVP：只做 US1 会让分类的折叠判重缺失，与 001 的既有约束不一致。

---

## 约束与提醒

- 单文件上限 700 行（宪法 I）；`src/store/goal.rs` 增加判重后接近 450 行，若本次改动让它越过 600 行预警线，先拆分再继续
- 性能预算的 `TODO(#1)` 已在 2026-11-04 的实测中清理并写入宪法第 V 条（1.18.0）；本 feature 不新增 TODO
- 不得为让 CI 变绿添加 lint 抑制；死代码豁免只接受 `#[expect(dead_code, reason = "...，见 #<issue>")]`
- 001 的 `tests/store_test.rs` 中有一条断言「删除后应允许重建同名分类」，本 feature 将其反向（T012），这是有意的行为变更，提交信息需说明
- 迁移会清空全部业务数据，T022 手工验收前先备份本地数据库文件
