# Tasks: 用图标替换操作按钮

**Input**: `specs/010-icon-buttons/` 的 spec、plan、research、data-model、contracts、quickstart

**Tests**: 宪法 III 要求测试先行；图标控件与替换的用例先写并观察到失败，再写最少实现

**Organization**: 按用户故事分阶段，每个阶段可独立验收

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3）
- 任务描述必须含精确文件路径

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 确认技术前提与逐按钮映射

- [ ] T001 核对当前分支为 `010-icon-buttons`、`Cargo.toml` 不新增依赖，并把 `contracts/icon-buttons.md` 的逐按钮映射作为替换时的唯一依据

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 图标控件与它的验收用例，所有故事都依赖它们

**⚠️ CRITICAL**: 本阶段完成前不开始任何用户故事

- [ ] T002 [P] 在 `src/app/tests/icons.rs` 写失败用例并在 `src/app/tests.rs` 注册 `mod icons;`：映射表每项形状与提示文案非空、危险语义只出现在危险操作、两套主题下含图标的界面渲染一帧不 panic、点击图标后业务状态与替换前一致
- [ ] T003 在 `src/app/icons.rs` 实现 `Icon` 枚举、`IconButton`（固定 34×26 命中区域、`on_hover_text`、普通／主要／危险语义）与 13 种线条形状绘制，颜色只取 `text_color()` / `error_fg_color()`
- [ ] T004 在 `src/app.rs` 挂载 `mod icons;`，并确认 `cargo clippy --all-targets --all-features -- -D warnings` 无死代码

---

## Phase 3: User Story 1 - 操作按钮改为图标 (Priority: P1)

**Purpose**: 把范围内的文字按钮换成图标，行为完全不变

**Independent Test**: 既有 headless 用例 + 新增图标点击用例；逐屏截图确认无文字残留

- [ ] T005 [US1] 在 `src/app/status_bar.rs` 与 `src/app/popup.rs` 把「统计」「知道了」「查看今日清单」换成 `IconButton`
- [ ] T006 [US1] 在 `src/app/main_view.rs` 把结构树、任务行与新建弹窗的全部操作按钮换成 `IconButton`，保留 `selectable_label` 选项与表单文字
- [ ] T007 [US1] 在 `src/app/today_view.rs` 把「完成」「跳过」「撤销」换成 `IconButton`，保持行动描述与状态文案为文字
- [ ] T008 [US1] 在 `src/app/rename_dialog.rs`、`frequency_dialog.rs`、`estimate_dialog.rs`、`delete_dialog.rs`、`reset_dialog.rs`、`stats_dialog.rs` 把保存／取消／删除／重置／关闭换成 `IconButton`
- [ ] T009 [US1] 在 `src/app/theme.rs` 删除替换后不再有消费者的 `danger` / `primary_button`，避免死代码

---

## Phase 4: User Story 2 - 图标仍然可理解 (Priority: P2)

**Purpose**: 悬停提示与保留文字的信息控件

**Independent Test**: 映射表逐项核对；`git grep` 确认文字控件未被误替换

- [ ] T010 [US2] 按 `contracts/icon-buttons.md` 逐项核对每个 `IconButton` 的悬停文案与语义级别，并确认主题三态下拉、表单标签、统计数值、空状态与错误提示仍是文字

---

## Phase 5: User Story 3 - 两套主题下图标清晰 (Priority: P3)

**Purpose**: 形状、语义色与布局在浅色／深色下都成立

**Independent Test**: `src/app/tests/icons.rs` 的双主题渲染用例 + 两套 GUI 截图

- [ ] T011 [US3] 在 `src/app/tests/icons.rs` 补用例：八个弹窗与今日清单在浅色、深色下各渲染一帧，且图标不引入新的 `Color32` 字面量

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T012 用隔离的 `%LOCALAPPDATA%` 起 GUI，按 `specs/010-icon-buttons/quickstart.md` 走场景 1~4，浅色／深色各截图并记录排版结论
- [ ] T013 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`，并用 `git grep -nE 'Color32::' -- src` 与 `git diff main -- Cargo.toml` 复核约束
- [ ] T014 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

---

## Dependencies

- T001 → T002；T002 → T003 → T004
- T004 → T005 / T006 / T007 / T008（四个文件互不重叠，可并行）
- T005 + T006 + T007 + T008 → T009（清理无消费者函数）
- T009 → T010 → T011
- T011 → T012 → T013 → T014

## Parallel Execution Examples

```text
Phase 3: T005 (status_bar/popup) ∥ T006 (main_view) ∥ T007 (today_view) ∥ T008 (六个弹窗)
```

四个任务改动的文件互不重叠；`src/app/theme.rs` 的清理必须等它们全部完成。

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 → Phase 2（图标控件与失败用例）
2. Phase 3：四个文件组并行替换 + 清理死代码
3. **STOP and VALIDATE**：既有 headless 用例与图标点击用例全部通过

### Incremental Delivery

1. Foundation ready → US1：按钮换图标
2. 加 US2：悬停提示与文字控件复核
3. 加 US3：双主题渲染与截图
4. Phase 6：截图、门禁、记录、PR

### Notes

- 所有测试任务必须先观察到失败，再写实现
- 每个阶段完成后立即提交；纯拆分/文档与功能改动分开提交
- 不新增依赖、不改业务数据、不改主题偏好键
- 替换后 `ui.button` / `ui.small_button` 在替换范围内不应再有残留，可用 `git grep` 复核
