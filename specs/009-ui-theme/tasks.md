# Tasks: 界面美化与明暗主题

**Input**: `specs/009-ui-theme/` 的 spec、plan、research、data-model、contracts、quickstart

**Tests**: 宪法 III 要求测试先行；每个新接口先写用例并观察到失败，再写最少实现

**Organization**: 按用户故事分阶段，每个阶段可独立验收

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3）
- 任务描述必须含精确文件路径

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 确认技术前提，不新增脚手架

- [x] T001 核对 `Cargo.toml` 不新增依赖、当前分支为 `009-ui-theme`，并把 egui 0.37 的 `ThemePreference` / `RawInput::system_theme` / `set_style_of` 结论记入 `specs/009-ui-theme/research.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 领域枚举、偏好存储与视觉令牌，三个故事都依赖它们

**⚠️ CRITICAL**: 本阶段完成前不开始任何用户故事

- [x] T002 [P] 在 `tests/theme_test.rs` 写失败用例：无记录时返回「跟随系统」、三种取值往返、未知值回退、写入 `theme` 不改变 `settings.daily_cap` 与业务表行数
- [x] T003 在 `src/model/theme.rs` 定义 `ThemeChoice`（`System` / `Light` / `Dark`，持久化字符串 `system` / `light` / `dark`，`parse` 对未知值返回 `System`），并在 `src/model.rs` 用 `mod theme` + `pub use` 转出
- [x] T004 在 `src/store/theme.rs` 实现 `theme_choice()` / `set_theme_choice()`（复用 `settings` 表 `theme` 键、参数绑定、`OpTrace`、契约注释），并在 `src/store.rs` 挂载模块
- [x] T005 [P] 在 `src/app/tests/theme.rs` 写失败用例：浅色与深色令牌的正文/弱化文字/警告/危险色对比度 ≥ 4.5:1、大字号 ≥ 3:1；切换主题前后 `actions` / `tasks` / `stats` 快照一致
- [x] T006 在 `src/app/theme.rs` 定义两套 `egui::Style`（语义色、间距、字号层级）、`contrast_ratio` 与 `apply_styles(ctx)`；界面其余文件不得出现颜色字面量

---

## Phase 3: User Story 1 - 切换明暗主题 (Priority: P1)

**Purpose**: 顶栏一键切换，主界面与弹窗同步换色

**Independent Test**: headless 渲染切换前后状态，弹窗渲染不 panic；GUI 截图确认整体换色

- [x] T007 [US1] 在 `src/app/tests/theme.rs` 写失败用例：切换后 `theme` 字段与 `ctx.theme()` 一致、主界面与统计面板在两套主题下都能渲染、写库失败时错误提示且不改偏好
- [x] T008 [US1] 在 `src/app.rs` 增加 `theme: ThemeChoice` 字段与 `set_theme(ctx, choice)`：写库、`ctx.set_theme`、结构化日志、失败时只记一次错误
- [x] T009 [US1] 在 `src/app.rs` 顶栏增加主题入口：三态互斥选择并显示当前值，与「统计」同一区域
- [x] T010 [US1] 在 `src/run.rs` 的 `eframe` 创建回调里调用 `apply_styles(ctx)`，让首帧就具备两套语义色

---

## Phase 4: User Story 2 - 主题选择被记住 (Priority: P2)

**Purpose**: 重启恢复、默认跟随系统、运行中跟随系统变化

**Independent Test**: 跨 `Store` 实例读写；注入 `RawInput.system_theme` 观察 `ctx.theme()`；GUI 重启截图

- [x] T011 [US2] 在 `tests/theme_test.rs` 与 `src/app/tests/theme.rs` 写失败用例：偏好跨 `Store` 实例恢复、`ThemePreference::System` 下注入 `Some(Dark)` / `Some(Light)` 时 `ctx.theme()` 跟随、未知持久化值回退 `System`
- [x] T012 [US2] 在 `src/app.rs` 的构造流程读取偏好、映射为 `egui::ThemePreference`，未知值回退 `System` 并记一条警告，不写库
- [x] T013 [US2] 在 `src/run.rs` 创建回调中按已读偏好调用 `ctx.set_theme(...)`，早于首帧，保证重启后无可见闪烁

---

## Phase 5: User Story 3 - 视觉层级统一 (Priority: P3)

**Purpose**: 统一的间距、字号、操作分组、空状态与错误提示

**Independent Test**: 两套主题下的 headless 渲染用例 + GUI 截图逐屏核对

- [x] T014 [US3] [P] 在 `src/app/main_view.rs` 与 `src/app/today_view.rs` 用 `ui.weak()` / 令牌间距统一空状态、任务行列对齐与长名称截断
- [x] T015 [US3] [P] 在 `src/app/rename_dialog.rs`、`delete_dialog.rs`、`frequency_dialog.rs`、`estimate_dialog.rs`、`stats_dialog.rs` 统一标题、内边距与底部操作区；危险操作使用 `error_fg_color` 并保留二次确认
- [x] T016 [US3] 在 `src/app.rs` 把错误提示与「已顺延」改为 `error_fg_color` / `warn_fg_color`，删除两处 `Color32` 字面量
- [x] T017 [US3] 在 `src/app/tests/render.rs` 或 `src/app/tests/theme.rs` 补用例：五个弹窗在浅色与深色下各渲染一帧且不 panic

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T018 用隔离的 `%LOCALAPPDATA%` 起 GUI，按 `specs/009-ui-theme/quickstart.md` 走场景 1~4，浅色/深色各截图并记录排版结论
- [x] T019 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`，并用 `git grep -nE 'Color32::' -- src` 确认颜色字面量只在令牌文件
- [x] T020 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

---

## 验收记录

- **测试先行**：T002 / T005 / T007 / T011 的用例先落地并观察到编译失败（缺 `ThemeChoice`、缺 `theme_choice`、缺 `style_for`、缺 `apply_theme` / `set_theme`），再写最少实现；US3 的渲染用例是外观变更的回归防护，行为不变，随实现一起落地
- **GUI 截图**（隔离 `%LOCALAPPDATA%`，200 任务 × 90 天数据，release）：把 `settings.theme` 置为 `dark` / `light` 后各重启一次，`target/gui-009/dark.png` 与 `light.png` 显示顶栏分别为「深色」/「浅色」，主界面、任务行、统计面板与进度条同步换色；`×` 危险按钮在两套主题下均为语义红色，「已顺延」为语义警告色，无新旧配色混杂、无溢出
- **截图手段**：鼠标事件注入在该环境未被 egui 接收（与 006 / 008 的记录一致），改为截图阶段临时让 `MinActionApp::new` 自动打开统计面板；验收后已还原，`git diff` 确认源码零残留。主题选择通过直接改隔离数据库的 `settings.theme` 并重启验证，等价于走完 UI 入口后的重启恢复
- **未做的手工项**：未改 Windows 系统主题做人工跟随验证，也未单独截空状态与错误提示；跟随系统由 `RawInput.system_theme` 注入用例覆盖，首帧无闪烁由 `eframe` 创建回调早于首帧的设计论证，空状态与错误提示由两套主题的全弹窗渲染用例和令牌对比度用例覆盖
- **文件规模**：`src/app.rs` 在实现期间涨到 644 行，触发宪法 I 的 600 行预警，按职责拆出 `src/app/status_bar.rs` 后降到 552 行；所有 `.rs` 均低于 600 行
- **门禁**：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`（库内 39 条 + 集成 76 条）、`cargo machete`、`cargo audit` 全部通过；`git grep -nE 'Color32::' -- src` 只在 `src/app/theme.rs` 命中

## Dependencies

- T001 → T002/T005；T002 → T003 → T004
- T005 → T006；T004 + T006 → T007
- T007 → T008 → T009 → T010
- T010 → T011 → T012 → T013
- T006 → T014 / T015 / T016 → T017
- T013 + T017 → T018 → T019 → T020

## Parallel Execution Examples

```text
Phase 2: T002 (tests/theme_test.rs) ∥ T005 (src/app/tests/theme.rs)
Phase 5: T014 (main_view/today_view) ∥ T015 (五个弹窗)
```

两个并行组内文件互不重叠；跨阶段仍按依赖顺序执行。

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 → Phase 2（阻塞前置）
2. Phase 3：顶栏切换 + 两套样式注册
3. **STOP and VALIDATE**：headless 用例与浅/深截图确认整体换色

### Incremental Delivery

1. Foundation ready → MVP（US1）
2. 加 US2：重启恢复与跟随系统 → 独立验收
3. 加 US3：视觉层级统一 → 独立验收
4. Phase 6：截图、门禁、记录、PR

### Notes

- 所有测试任务必须先观察到失败，再写实现
- 每个阶段完成后立即提交；纯拆分/文档与功能改动分开提交
- 不新增依赖；不改业务表结构；两套主题共用同一布局
- 涉及 `src/app.rs` 的多个任务共用同一文件，按顺序执行，不并行
