# Tasks: 结构树改用右键菜单

**Input**: `specs/011-context-menu/` 的 spec、plan

**Tests**: 宪法 III 要求测试先行；菜单用例先写并观察到失败，再写最少实现

**Organization**: 按用户故事分阶段

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3）

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 核对当前分支为 `011-context-menu`，范围只含 `src/app/main_view.rs` 结构树目标／分类行，并确认 `Cargo.toml` 不新增依赖

---

## Phase 2: Foundational (Blocking Prerequisites)

- [x] T002 [P] 在 `src/app/tests/context_menu.rs` 写失败用例并在 `src/app/tests.rs` 注册 `mod context_menu;`：菜单项恰好为「改名」「删除」、动作齐全、两项在浅色与深色下渲染一帧不 panic
- [x] T003 在 `src/app/structure_menu.rs` 实现 `StructureAction`、`MENU_ITEMS` 常量表与 `show(ui)`；删除项用 `error_fg_color()`，点击后 `ui.close()`；在 `src/app.rs` 挂载 `mod structure_menu;`

---

## Phase 3: User Story 1 - 右键菜单执行改名与删除 (Priority: P1)

- [x] T004 [US1] 在 `src/app/main_view.rs` 的目标行：去掉 `Rename` / `DeleteGoal` 两个图标按钮，名称控件接 `context_menu` 与悬停提示，左键仍走 `select_goal`
- [x] T005 [US1] 在 `src/app/main_view.rs` 的分类行：同样去掉 `Rename` / `DeleteCategory`，菜单动作写入 `rename` / `drop_category`，左键仍走 `select_category`

---

## Phase 4: User Story 2 - 行内按钮移除且交互不退化 (Priority: P2)

- [x] T006 [US2] 在 `src/app/icons.rs` 删除失去消费者的 `ButtonId::DeleteGoal` / `DeleteCategory` 变体、`ALL` 条目与 `spec()` 分支，并同步更新 `src/app/tests/icons.rs` 中危险语义数量断言
- [x] T007 [US2] 用 `git grep` 复核结构树行内无改名／删除图标调用，展开箭头、顶栏图标、任务行与今日清单图标按钮未改动

---

## Phase 5: User Story 3 - 菜单在两套主题下可读 (Priority: P3)

- [x] T008 [US3] 在 `src/app/tests/context_menu.rs` 补用例：菜单在浅色与深色两套 `Style` 下渲染，删除项取语义危险色且不引入新的 `Color32` 字面量

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T009 用隔离的 `%LOCALAPPDATA%` 起 GUI，浅色／深色各截一张结构树与右键菜单，确认行内按钮已移除、菜单文字可读、无溢出
- [x] T010 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`，并复核 `git diff main -- Cargo.toml` 与颜色字面量
- [x] T011 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

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

## 验收记录（2026-11-06）

### 产物与范围

| 项 | 结果 |
| --- | --- |
| 分支 | `011-context-menu`，基于 `main` 的 `1a4d0ea`（010） |
| 新增 | `src/app/structure_menu.rs`、`src/app/tests/context_menu.rs` |
| 修改 | `src/app/main_view.rs`（目标／分类行）、`src/app/icons.rs`（删两个 `ButtonId`）、`src/app/tests/icons.rs`（危险语义数 5→3）、`src/app/tests.rs`（注册与说明）、`src/app.rs`（挂载模块） |
| 设计文档 | 按 `plan.md` 的裁剪说明，不生成 `research.md` / `data-model.md` / `contracts/` / `quickstart.md`；宪法未要求固定数量 |
| 依赖与数据 | `git diff main -- Cargo.toml` 为空；业务表、`settings` 键、调度逻辑未改 |

### 测试先行

- T002 先写用例并观察到 `unresolved import crate::app::structure_menu` 失败，再补 `structure_menu.rs` 与模块挂载后转绿。
- `main_view.rs` 两处 `context_menu` 与 `on_hover_text("左键选中，右键菜单")` 上线后，既有 47 条 lib 用例通过；新增合成右键事件用例后共 48 条。

### GUI 截图（T009）

- 隔离 `%LOCALAPPDATA%` 起 GUI，用 `PrintWindow(..., 2)` 按窗口标题「最小行动」抓主窗口；截图保存在 `target/gui-011/dark-main.png` 与 `light-main.png`（`target/` 不入库）。
- 两张图均显示：目标／分类行只剩展开箭头与名称，红框内的改名／删除图标按钮已消失；名称、选中态、任务区在浅色与深色下可读，无溢出。
- **偏差**：本环境的输入注入无效——`PostMessage` 与真实鼠标事件都无法驱动该窗口，连左键切换选中分类都不生效（已用对照截图验证）。因此无法截到「菜单已弹出」的真实画面，SC-001 中菜单可读性一项改由 headless 用例覆盖：
  - `name_context_menu_opens_on_secondary_click`：合成 `PointerMoved` + 右键按下／抬起事件，断言 `Response::context_menu` 的闭包在后续帧执行，证明菜单确实会打开；
  - `menu_renders_in_both_themes` 与 `structure_tree_with_context_menus_renders_in_both_themes`：浅色／深色各渲染一帧；
  - `delete_item_uses_the_semantic_error_color`：两套主题下 `error_fg_color` 与 `text_color` 不同，删除项取语义危险色。
- 验收用的临时脚本（`target/theme_set.py`、注入脚本）已删除，源码零残留。

### 门禁（T010）

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --all-targets --all-features` | PASS（lib 48 + 集成测试 + benchmark 冒烟） |
| `cargo machete` | PASS，无未使用依赖 |
| `cargo audit` | PASS（5 条 allowed warnings，与 `main` 一致） |
| `git diff main -- Cargo.toml` | 空 |
| `git grep 'Color32::' -- src` | 仅 `src/app/theme.rs`（009 令牌文件） |
| `git grep 'DeleteGoal\|DeleteCategory' -- src` | 无命中 |
| `git grep '#\[(allow\|expect)(' -- src` | 无命中，无 lint 抑制 |
| 裸 `TODO` / `FIXME` | 源码无命中 |
| 文件规模（宪法 I） | 最大 `src/app.rs` 515 行，全部 `.rs` < 600 行预警线 |

### 宪法复核

八项原则与 `plan.md` 的 Constitution Check 一致，全部 PASS；唯一偏差是上述 GUI 菜单截图，属于验证手段替换，不产生治理例外，也不新增配置、开关或抽象（VI）。
