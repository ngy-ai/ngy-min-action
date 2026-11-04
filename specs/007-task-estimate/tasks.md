# Tasks: 任务预计耗时

**Input**: `specs/007-task-estimate/` 的 spec、plan
**Tests**: 存储方法先写失败用例；界面沿用 004 的 headless 验收

## Phase 1: Foundational

- [ ] T001 在 `tests/store_test.rs` 写入失败用例：设置耗时、清空耗时、已删除任务返回 `NotFound`
- [ ] T002 在 `src/store/task.rs` 实现 `update_task_estimated_minutes`，单条 `UPDATE`，复用 `OpTrace`

## Phase 2: User Story 1 - 查看并修改任务耗时 (Priority: P1)

- [ ] T003 在 `src/app.rs` 增加 `estimate_target: Option<TaskId>` 与 `draft_estimate: String`
- [ ] T004 新建 `src/app/estimate_dialog.rs`：预填当前值，保存走 `validate_estimated_minutes`，失败保留弹窗
- [ ] T005 在 `src/app/main_view.rs` 的任务行显示耗时标签（仅在有值时），点击打开弹窗

## Phase 3: User Story 2 - 新建任务时填耗时 (Priority: P2)

- [ ] T006 在 `src/app.rs` 增加 `new_task_estimate: String`
- [ ] T007 在 `src/app/main_view.rs` 的新建表单加耗时输入，创建任务时解析并校验；成功后清空

## Phase 4: 验收

- [ ] T008 新建 `src/app/tests/estimate.rs`：改值与清空、非法值保留弹窗、改耗时不动频率与进度
- [ ] T009 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`
- [ ] T010 用隔离的 `%LOCALAPPDATA%` 起一次 GUI 截图，确认有值任务的耗时标签可见、无值任务不占位

## Dependencies

- T001 → T002
- T003 → T004 → T005
- T006 → T007
- T008 依赖 T004/T005/T007

## 约束与提醒

- `main_view.rs` 当前 451 行，本次只加输入与标签，约束在 480 行内
- 留空与 `0` 都表示「未估时」，不要在界面上显示「0 分钟」
- 新增测试放 `src/app/tests/estimate.rs`，需在 `tests.rs` 里声明子模块
- 优先级：T001–T005 完成即可交付 P1；T006/T007 是 P2，可拆两个提交
