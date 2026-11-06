# Tasks: 结构树改用右键菜单

**Input**: `specs/011-context-menu/` 的 spec、plan

**Tests**: 宪法 III 要求测试先行；菜单用例先写并观察到失败，再写最少实现

**Organization**: 按用户故事分阶段

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3）

## Phase 1: Setup (Shared Infrastructure)

- [ ] T001 核对当前分支为 `011-context-menu`，范围只含 `src/app/main_view.rs` 结构树目标／分类行，并确认 `Cargo.toml` 不新增依赖

---

## Phase 2: Foundational (Blocking Prerequisites)

- [ ] T002 [P] 在 `src/app/tests/context_menu.rs` 写失败用例并在 `src/app/tests.rs` 注册 `mod context_menu;`：菜单项恰好为「改名」「删除」、动作齐全、两项在浅色与深色下渲染一帧不 panic
- [ ] T003 在 `src/app/structure_menu.rs` 实现 `StructureAction`、`MENU_ITEMS` 常量表与 `show(ui)`；删除项用 `error_fg_color()`，点击后 `ui.close()`；在 `src/app.rs` 挂载 `mod structure_menu;`

---

## Phase 3: User Story 1 - 右键菜单执行改名与删除 (Priority: P1)

- [ ] T004 [US1] 在 `src/app/main_view.rs` 的目标行：去掉 `Rename` / `DeleteGoal` 两个图标按钮，名称控件接 `context_menu` 与悬停提示，左键仍走 `select_goal`
- [ ] T005 [US1] 在 `src/app/main_view.rs` 的分类行：同样去掉 `Rename` / `DeleteCategory`，菜单动作写入 `rename` / `drop_category`，左键仍走 `select_category`

---

## Phase 4: User Story 2 - 行内按钮移除且交互不退化 (Priority: P2)

- [ ] T006 [US2] 在 `src/app/icons.rs` 删除失去消费者的 `ButtonId::DeleteGoal` / `DeleteCategory` 变体、`ALL` 条目与 `spec()` 分支，并同步更新 `src/app/tests/icons.rs` 中危险语义数量断言
- [ ] T007 [US2] 用 `git grep` 复核结构树行内无改名／删除图标调用，展开箭头、顶栏图标、任务行与今日清单图标按钮未改动

---

## Phase 5: User Story 3 - 菜单在两套主题下可读 (Priority: P3)

- [ ] T008 [US3] 在 `src/app/tests/context_menu.rs` 补用例：菜单在浅色与深色两套 `Style` 下渲染，删除项取语义危险色且不引入新的 `Color32` 字面量

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T009 用隔离的 `%LOCALAPPDATA%` 起 GUI，浅色／深色各截一张结构树与右键菜单，确认行内按钮已移除、菜单文字可读、无溢出
- [ ] T010 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`，并复核 `git diff main -- Cargo.toml` 与颜色字面量
- [ ] T011 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

---

## Dependencies

- T001 → T002 → T003
- T003 → T004 → T005
- T005 → T006 → T007 → T008
- T008 → T009 → T010 → T011

## Implementation Strategy

1. Phase 1 → Phase 2：菜单定义与失败用例
2. Phase 3：目标行与分类行替换（同一文件，顺序执行）
3. Phase 4：清理图标映射、复核其它按钮未动
4. Phase 5 + Phase 6：双主题用例、截图、门禁、记录、PR

## Notes

- 菜单项用文字标签，属于新交互，不算 010 工具条图标化范围的回退
- `context_menu` 闭包沿用 `ui_main_view` 已有的局部状态变量，不新增状态字段
- 每个阶段完成后立即提交；文档与实现分开提交
