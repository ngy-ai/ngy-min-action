# Tasks: 重命名与任务删除的操作入口

**Input**: `specs/004-edit-and-delete/` 的 spec、plan、contracts
**Tests**: 计数查询先写失败用例；界面靠手工验收

## Phase 1: Foundational

- [x] T001 在 `tests/store_test.rs` 写入失败用例：`task_open_actions` 统计 pending 与 deferred、不统计 done 与 skipped、已删除任务返回 0
- [x] T002 在 `src/store/task.rs` 实现 `task_open_actions`，单条 `COUNT(*)`，复用 `OpTrace`

## Phase 2: User Story 1 - 重命名目标与分类 (Priority: P1)

- [x] T003 在 `src/app.rs` 增加 `RenameTarget` 枚举（`Goal` / `Category` / `Task`）与 `draft_rename_name` 状态，并在 `new` 中初始化
- [x] T004 在 `src/app/main_view.rs` 的目标行与分类行加「改名」按钮，打开弹窗时预填当前名称
- [x] T005 在 `src/app/main_view.rs` 实现共用的重命名弹窗：标题按对象区分，提交按对象分派到 `rename_goal` / `rename_category` / `rename_task`，失败保留弹窗

## Phase 3: User Story 2 - 重命名任务 (Priority: P2)

- [x] T006 在 `src/app/main_view.rs` 的任务行加「改名」按钮，复用 T005 的弹窗

## Phase 4: User Story 3 - 删除任务 (Priority: P1)

- [x] T007 在 `src/app.rs` 的 `PendingDelete` 增加 `Task(TaskId)` 变体
- [x] T008 在 `src/app/main_view.rs` 的任务行加「删除」按钮，点击时读取 `task_open_actions` 并弹出确认框，文案说明将作废的行动数
- [x] T009 在 `src/app/main_view.rs` 的确认分支处理任务删除：调用 `delete_task` 后刷新

## Phase 5: Polish

- [ ] T010 按 `quickstart.md` 走完五个场景并记录结果
- [x] T011 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`

## Dependencies

- T001 → T002 → 后续界面任务
- T003 → T004/T005 → T006
- T007 → T008 → T009
- 界面任务同在 `main_view.rs`，串行

## 约束与提醒

- `main_view.rs` 当前约 500 行，本次预计 +120 行；越过 600 行预警线就把弹窗拆到独立模块
- 三类对象共用一个弹窗，不写三套
- 任务名不要求唯一，不要复用目标/分类的唯一性校验
