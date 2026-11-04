# Tasks: 任务行布局与误触防护

**Input**: `specs/006-task-row-actions/` 的 spec、plan
**Tests**: 先写失败用例，再实现；界面沿用 004 的 headless 验收

## Phase 1: User Story 1 - 重置复习前二次确认 (Priority: P1)

- [ ] T001 在 `src/app.rs` 增加 `reset_target: Option<TaskId>` 状态并在 `new` 中初始化
- [ ] T002 在 `src/app/tests.rs` 写入失败用例：打开确认不改进度、确认后归零、取消后不变、任务已删除时弹窗保留
- [ ] T003 新建 `src/app/reset_dialog.rs`：展示任务名与当前级别，确认走 `reset_review_progress`，失败保留弹窗
- [ ] T004 在 `src/app/main_view.rs` 把「重置复习」按钮改为打开确认弹窗

## Phase 2: User Story 2 - 频率标签即入口 (Priority: P2)

- [ ] T005 在 `src/app/main_view.rs` 把频率标签改为可点击控件，悬停提示「修改频率」，点击打开改频率弹窗
- [ ] T006 移除任务行的「频率」按钮

## Phase 3: User Story 3 - 删除入口分隔 (Priority: P3)

- [ ] T007 在任务行的 `×` 前加 `ui.add_space(8.0)`

## Phase 4: Polish

- [ ] T008 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`
- [ ] T009 用隔离的 `%LOCALAPPDATA%` 起一次 GUI 截图，确认任务行只剩 4 个控件且间距可见

## Dependencies

- T001 → T002 → T003 → T004
- T005/T006/T007 互相独立

## 约束与提醒

- 不新建通用确认组件，三个弹窗保持各自模块
- 重置只改 `review_progress`，不碰 `actions`
- 既有 17 条界面用例必须保持通过
