# Tasks: 待办工作区

**Input**: `specs/012-todo-workspace/` 的 spec、plan

**Tests**: 宪法 III 要求测试先行；每个阶段先写并观察到失败，再写最少实现

**Organization**: 先数据与迁移地基，再按用户故事接入界面

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3 / US4）

---

## Phase 1: Setup (Shared Infrastructure)

- [ ] T001 核对分支为 `012-todo-workspace`，确认 `Cargo.toml` 仅新增 `rfd`、`egui_extras`（image）、`image`（png/jpeg/webp/bmp）与 `rusqlite` 的 `backup` feature
- [ ] T002 先跑一次 `cargo fmt --check` / `clippy` / `test` 记录基线，确保后续失败只来自本 feature

---

## Phase 2: Foundational (Blocking Prerequisites)

- [ ] T003 [P] 在 `tests/note_test.rs` 写失败用例：schema 2 库升级到 3 后既有行动保留、`note` 默认空；备注设置/清空/超长拒绝；图片行增删与按待办查询；历史游标分页无重复
- [ ] T004 实现 `src/store/schema.rs` 的 `SCHEMA_VERSION = 3` 与 2→3 增量迁移、`actions.note` 列、`action_images` 表与索引；新增 `src/store/note.rs` 与 `actions_before`，让 T003 转绿
- [ ] T005 [P] 在 `tests/storage_test.rs` 写失败用例：`location` 缺失/非法回退默认、绝对路径往返、迁移拒绝同目录/目标已有数据/不可写、成功后附件完整、失败保留原位置
- [ ] T006 实现 `src/config.rs`（启动目录、`location` 原子读写）与 `src/attachment.rs`（格式/大小校验、复制、删除、缺失占位）
- [ ] T007 实现 `src/storage.rs` 迁移编排：`Store::backup_to`、附件递归复制、`integrity_check`、写配置、清理旧目录；`src/run.rs` 改用启动目录锁与可配置数据目录

---

## Phase 3: User Story 1 - 当日待办与图文备注 (Priority: P1)

- [ ] T008 [US1] 在 `src/app/tests/note.rs` 写失败用例：备注保存/清空走同一入口、图片加入后数据目录出现副本、移除后文件消失、空备注不写行
- [ ] T009 [US1] 实现 `src/app/note_dialog.rs`：文字上限、保存、文件选择、拖拽加入、缩略图、逐张移除；在 `src/app/today_view.rs` 加备注入口并显示备注/图片标记
- [ ] T010 [US1] 在 `src/run.rs` 安装 `egui_extras` 图片加载器；缺失文件与损坏文件走占位路径，不 panic

---

## Phase 4: User Story 2 - 历史待办查看与管理 (Priority: P1)

- [ ] T011 [US2] 在 `src/app/tests/history.rs` 写失败用例：历史按日期倒序、页大小固定、翻页无重复、空历史文案、备注只读展示
- [ ] T012 [US2] 实现 `src/app/history_view.rs` 与页签状态 `TodoTab`：分页游标、完成/跳过/撤销复用既有结算入口，切换页签不重置游标

---

## Phase 5: User Story 3 - 菜单栏与状态栏 (Priority: P2)

- [ ] T013 [US3] 在 `src/app/tests/workspace.rs` 写失败用例：菜单三组齐全、视图项切换页签、状态栏含路径/计数/版本、长路径省略、双主题渲染
- [ ] T014 [US3] 把现有 `src/app/status_bar.rs` 纯重命名拆分为 `toolbar.rs`（独立提交，不改行为）
- [ ] T015 [US3] 实现 `src/app/menu_bar.rs` 与新的 `src/app/status_bar.rs`，在 `src/app.rs` 装配菜单栏、工具条、页签与底栏

---

## Phase 6: User Story 4 - 存储位置设置与立即迁移 (Priority: P2)

- [ ] T016 [US4] 在 `src/app/tests/storage_dialog.rs` 写失败用例：当前路径展示、选择目标后可迁移、迁移成功刷新路径、失败保留原路径并显示可执行错误
- [ ] T017 [US4] 实现 `src/app/storage_dialog.rs`：`rfd` 目录选择、迁移中禁用、成功/失败状态；在「文件」菜单挂入口，状态栏同步新路径

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T018 用隔离 `%LOCALAPPDATA%` 起 GUI：浅色/深色各截菜单栏、状态栏、今日与历史页签、备注弹窗；再迁移到临时目录，重启确认数据与图片来自新位置
- [ ] T019 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`；复核 `Color32::`、文件规模、lint 抑制与裸 TODO
- [ ] T020 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

---

## Dependencies

- T001 → T002 → T003 → T004
- T004 → T005 → T006 → T007
- T007 → T008 → T009 → T010
- T004 → T011 → T012
- T012 → T013 → T014 → T015
- T007 → T016 → T017
- T010 / T015 / T017 → T018 → T019 → T020

## Implementation Strategy

1. Phase 1-2：先打通数据、图片文件与迁移，界面之前就能用测试证明契约
2. Phase 3：今日待办备注与图片，完成后 US1 独立可用
3. Phase 4：历史分页与结算，复用既有 `settle_action` / `undo_last_settlement`
4. Phase 5：菜单栏、状态栏与页签外壳；`toolbar` 重命名单独提交
5. Phase 6：存储设置弹窗与迁移入口
6. Phase 7：截图、门禁、记录与 PR

## Notes

- 迁移成功后必须先替换 `Store` 再清理旧目录；旧连接未释放前不得删除源文件
- 图片加入顺序：先校验并复制到临时名，再写库；任一步失败都清理副本
- 历史页只做只读备注，结算按钮复用今日页的同一套方法，避免两套规则
- 菜单项用文字，不扩展 010 的图标映射；新增图标只在确有工具条按钮时加
