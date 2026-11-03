# Tasks: 任务频率选择与复习进度展示

**Input**: `specs/003-task-form-and-progress/` 的 spec、plan、contracts
**Tests**: 领域与存储层先写失败用例；界面展示靠手工验收（沿用 001 的 D 决策）

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

无新增依赖、无表结构变更。直接进入用户故事。

## Phase 2: User Story 1 - 创建任务时选择频率 (Priority: P1) 🎯 MVP

**Independent Test**: 同一分类下创建每天、每 3 天、每周 2 次三个任务，读回频率与所选一致；越界参数被拒。

- [X] T001 [US1] 在 `tests/store_test.rs` 写入失败用例：`Frequency::label` 三种输出、`create_task` 按所选频率入库并读回、`from_parts` 对 0 与越界值拒绝
- [X] T002 [US1] 在 `src/model.rs` 实现 `Frequency::label`，契约注释写明「只用于展示，不参与调度判定」
- [X] T003 [US1] 在 `src/app/main_view.rs` 的新建任务表单加频率模式选择与参数输入：默认每天；后两种模式显示参数框；切换模式时重置为该模式默认值（3 天 / 2 次）
- [X] T004 [US1] 在 `src/app.rs` 接线提交：把界面选择的频率写入 `TaskDraft.frequency`，错误沿用现有错误条

**Checkpoint**: 三种频率都能创建，越界参数被拒且表单内容保留

## Phase 3: User Story 2 - 任务行显示频率 (Priority: P2)

**Independent Test**: 三种频率的任务分别显示「每天」「每 3 天」「每周 2 次」。

- [X] T005 [US2] 在 `src/app/main_view.rs` 的任务行用 `Frequency::label` 渲染频率，与任务名同行

**Checkpoint**: 频率在界面上可见

## Phase 4: User Story 3 - 任务行显示复习进度 (Priority: P1)

**Independent Test**: 完成任务后任务行显示「第 1 级 + 次日日期」；重置后回到「未开始」。

- [X] T006 [US3] 在 `tests/store_test.rs` 写入失败用例：`progress_for_category` 返回该分类全部未删除任务的进度、空分类返回空向量、已删除任务的进度不出现
- [X] T007 [US3] 在 `src/store/task.rs` 实现 `progress_for_category`，用 JOIN 一次取回，复用 `OpTrace`
- [X] T008 [US3] 在 `src/app.rs` 刷新任务列表时一并装入进度
- [X] T009 [US3] 在 `src/app/main_view.rs` 的任务行按三态渲染：无记录 →「未开始」；复习中 →「第 N 级，下次复习 YYYY-MM-DD」；已巩固 →「已巩固」，不显示下次复习日
- [X] T010 [US3] 确认「重置复习」后进度展示立即同步为未开始（必要时在重置后刷新任务列表）

**Checkpoint**: 三态展示与持久化状态一致

## Phase 5: Polish

- [X] T011 按 `quickstart.md` 走完四个场景并逐条记录结果（2026-11-03）
  - **场景 1** ✅：默认每天、每 3 天、每周 2 次均创建成功，任务行分别显示「每天」「每 3 天」「每周 2 次」
  - **场景 2** ✅：每周次数填 8 提交被拒，错误条「每周次数 不合法：期望 1 到 7 之间的整数」，任务名与参数保留、任务未创建；每 N 天的 0 / 366 由 `frequency_parameters_are_bounded` 单元测试覆盖
  - **场景 3** ✅：新任务显示「未开始」；完成行动后显示「第 1 级，下次复习 2026-11-04」；点击「重置复习」立即回到「未开始」
  - **场景 4** ✅：暂停后按钮变「恢复」，频率与进度展示不变
  - **验收中修正**：新任务落库即有 `level = 0` 的进度行，初版展示为「第 0 级」，已改为按未开始渲染
  - **新发现偏差（不属本 feature）**：暂停任务后，今日清单中该行动的名称退化为「·任务#1」，疑为名称解析只查 active 任务所致，需单独立项
- [X] T012 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`

## Dependencies

- US1 → US2：频率文案由 US1 的 `label` 提供
- US3 独立于 US1/US2，可与 US1 并行推进，但测试都在 `tests/store_test.rs`，实现都在 `src/app/main_view.rs`，实际串行
- 同文件任务不得并行

## 约束与提醒

- `src/app/main_view.rs` 当前 471 行，本 feature 预计增加约 80 行，若越过 600 行预警线先拆分
- 不新增「编辑已有任务频率」入口（规格 Assumptions）
- 界面不做自动化测试，靠 T011 手工验收，沿用 001 的 D 决策
