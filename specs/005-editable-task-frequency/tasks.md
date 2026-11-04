# Tasks: 任务频率可修改

**Input**: `specs/005-editable-task-frequency/` 的 spec、plan、contracts
**Tests**: 存储方法先写失败用例；界面沿用 004 的 headless 验收

## Phase 1: Foundational

- [ ] T001 在 `tests/store_test.rs` 写入失败用例：`update_task_frequency` 改成功且不动其他字段、软删除任务返回 `NotFound`
- [ ] T002 在 `src/store/task.rs` 实现 `update_task_frequency`，单条 `UPDATE`，复用 `OpTrace`

## Phase 2: User Story 1 - 修改任务频率 (Priority: P1)

- [ ] T003 在 `src/app.rs` 增加 `frequency_target`、`draft_frequency_mode`、`draft_frequency_interval` 状态并在 `new` 中初始化
- [ ] T004 在 `src/app/main_view.rs` 的任务行加「频率」按钮，点击时预填当前模式与参数
- [ ] T005 新建 `src/app/frequency_dialog.rs`：模式切换、参数输入、保存与取消；提交走 `Frequency::from_parts` → `update_task_frequency`，失败保留弹窗与输入

## Phase 3: User Story 2 - 当日清单按新频率推进 (Priority: P2)

- [ ] T006 在 `src/app/tests.rs` 补用例：改频率后任务行标签更新、名称与复习进度不变、当天已生成的行动不变
- [ ] T007 在 `src/app/tests.rs` 补跨日用例：从「每周 1 次」改成「每天」后，次日刷新按新频率生成行动

## Phase 4: Polish

- [ ] T008 按 `quickstart.md` 走完五个场景并记录结果
- [ ] T009 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`

## Dependencies

- T001 → T002 → 后续界面任务
- T003 → T004/T005 → T006/T007

## 约束与提醒

- `main_view.rs` 当前 444 行，本次只加按钮与请求收集，弹窗独立成模块
- 弹窗复用 `FrequencyMode` 与 `Frequency`，不新造枚举
- 频率控件在新建表单与弹窗中出现两次，按原则 VI 不提取共用组件
- T007 需要推进时钟，测试辅助函数要能拿到 `FixedClock`
