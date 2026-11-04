# Tasks: 复习统计与回顾

**Input**: `specs/008-review-stats/` 的 spec、plan
**Tests**: 存储方法先写失败用例；界面沿用 004 的 headless 验收

## Phase 1: Foundational

- [x] T001 在 `src/model.rs` 增加 `ActionTally`（含 `settled` 与 `completion_percent`）与 `ReviewStats`，并在 `lib.rs` 转出
- [x] T002 在 `tests/store_test.rs` 写入失败用例：分项计数、完成率、空窗口返回 `None`
- [x] T003 在 `tests/store_test.rs` 写入连续天数四种情形：连续达成、中间跳过日中断、中间无行动日不中断、今天零完成不中断
- [x] T004 在 `tests/store_test.rs` 写入阶段分布（含软删除排除）
- [x] T005 实现 `src/store/stats.rs::review_stats`，三条查询 + 线性遍历算 streak

## Phase 2: User Story 1 - 近 30 天完成率 (Priority: P1)

- [x] T006 在 `src/app.rs` 增加 `show_stats` 与 `stats: Option<ReviewStats>`
- [x] T007 新建 `src/app/stats_dialog.rs`：打开时查询并缓存，展示完成率与三条分项；无已结算时显示空状态

## Phase 3: User Story 2 - 连续天数 (Priority: P2)

- [x] T008 在面板中展示当前连续与最长连续，两者都为 0 时不显示「最长」

## Phase 4: User Story 3 - 复习阶段分布 (Priority: P3)

- [x] T009 在面板中展示未开始 / 学习中 / 已巩固三档计数与合计

## Phase 5: 验收

- [x] T010 在 `src/app/tests/stats.rs` 覆盖：打开后快照非空、空库显示空状态、打开不产生写入
- [x] T011 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`
- [x] T012 用隔离的 `%LOCALAPPDATA%` 起一次 GUI 截图，确认统计面板的排版

## Dependencies

- T001 → T002/T003/T004 → T005
- T005 → T006 → T007 → T008/T009
- T010 依赖 T007

## 验收记录

- 隔离 `%LOCALAPPDATA%` 启动 GUI，造 3 条今日行动（2 完成 / 1 跳过）后截图确认有数据面板：`近 30 天` 下进度条铺满 420 宽度并只填充约 2/3，文字「完成率 67%」；分项「完成 2 · 跳过 1 · 过期 0」；「当前连续 1 天 / 历史最长 1 天」；「未开始 1 · 学习中 2 · 已巩固 0 / 共 3 个任务」；「关闭」按钮完整可见，无截断或溢出
- 空库截图确认空态：显示「近 30 天还没有已结算的行动」，不显示进度条、历史最长与合计，「还没有任务」，「关闭」完整可见
- 鼠标事件注入在该环境仍未被 egui 接收（与 006 的记录一致）。截图阶段临时让 `MinActionApp::new` 自动打开面板，验收后已还原，该改动未入库；点击入口的行为由 `ui_top_bar` 的 `open_stats()` 与 headless 用例覆盖

## 约束与提醒

- 界面不做除法：完成率由 `ActionTally::completion_percent()` 给出
- 统计只读，任何实现都不得写库
- 不引入图表库，纯文本 + `ProgressBar` 即可
- 新增测试放 `src/app/tests/stats.rs`，需在 `tests.rs` 声明子模块
