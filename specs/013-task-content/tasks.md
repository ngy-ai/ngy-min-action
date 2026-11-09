# 013 任务清单

## Phase 1: 基础（数据模型、迁移、存储）

- [x] T001 在 `spec.md` / `plan.md` / `tasks.md` 记录需求、宪法复核与复杂度追踪
- [x] T002 模型：`Task` 增加 `description: Option<String>`，新增 `TaskMedia` / `TaskMediaKind` /
      `TaskContentMediaId`，`TaskDraft` 增加 `description`；新增 `MAX_TASK_DESCRIPTION` 与
      `validate_description`
- [x] T003 先写失败用例：内容校验、按 `id` 取任务、描述读写、媒体增删
- [x] T004 schema 3 → 4：`tasks.description` + `task_content_media` 表，2→3 之外的旧库清空重建，
      保留业务数据
- [x] T005 实现 `task(id)`、`task_description` / `set_task_description`、
      `add_task_media` / `task_media` / `delete_task_media`（先红后绿）
- [x] T006 `attachment.rs` 增加 `MediaStore`：图片走解码校验，音视频走扩展名 + 大小白名单；
      复制落盘与单段文件名校验

## Phase 2: 编辑入口

- [x] T007 新增 `app/task_dialog.rs`：任务内容编辑——多行 `TextEdit` + 媒体「添加 / 删除」
      （`rfd` 选文件），从任务区打开
- [x] T008 失败用例：内容保存/清空、媒体增删、非法文件被拒；保存后界面刷新

## Phase 3: 富内容渲染

- [x] T009 新增渲染助手：文本自动识别 `http(s)://` 为可点击链接；媒体块按类型——图片内联、
      音视频「打开」按钮（OS 默认程序）
- [x] T010 失败用例：链接解析、媒体类型映射、空内容

## Phase 4: 备注弹窗两栏

- [x] T011 `note_dialog.rs` 改左右两栏：左 = 来源任务（标题 + 渲染内容），右 = 当前备注；
      只读态两栏均只读
- [x] T012 失败用例：两栏结构渲染、来源任务标题/内容显示、只读态

## Phase 5: 验证与交付

- [ ] T013 隔离 `%LOCALAPPDATA%` 起 GUI，深浅截图：任务内容编辑、备注两栏（含图片/链接）、
      音视频打开按钮（本会话未执行，见验收记录）
- [x] T014 门禁：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、
      `cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`；复核
      `git diff main -- Cargo.toml` 与颜色字面量（全绿）
- [x] T015 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者
      仓库自我证明）；PR #20（base = `012-todo-workspace`，堆叠，待 #19 合入后改 base 为 `main`）

## Notes

- `MediaStore` 与既有 `AttachmentStore` 共享 `is_single_component` / `copy_new` / `generated_name`
  私有助手，避免重复。
- 迁移（`storage.rs`）按整目录复制数据目录，`task_content/` 随附件一起移动，无需特殊处理。

---

## 验收记录（2026-11-09）

### 产物与范围

| 项 | 结果 |
| --- | --- |
| 分支 | `013-task-content`，基于 `012-todo-workspace` 顶端（main 尚未合入 012，故 `git diff main` 同时含 012 的改动） |
| 提交 | `eef2fec` 规格/计划/任务、`20744ea` 数据层、`a8e2c5f` UI 实现、`db717ca` 测试、`40c7c11` 验收记录 |
| PR | #20（base = `012-todo-workspace`，堆叠；#19 合入 `main` 后改 base 为 `main`） |
| 新增（源码） | `src/app/rich_content.rs`、`src/app/task_dialog.rs` |
| 新增（测试） | `src/app/tests/task_content.rs`（集成 `tests/task_content_test.rs` 随数据层提交） |
| 修改 | `src/app.rs`、`src/app/{icons,main_view,note_dialog,tests}.rs`、`src/{model,common}.rs`、`src/common/constants.rs` |
| 依赖 | **013 未新增依赖**，沿用 012 的 `egui_extras`（`image`/`file`）、`image`、`rfd`、`rusqlite`（`backup`） |
| 数据 | schema 3 → 4 增量迁移：`tasks.description` 列 + `task_content_media` 表；2→3 之外旧库清空重建、保留业务数据 |
| 设计文档 | 按 `plan.md` 裁剪，未生成 `research.md` / `data-model.md` / `contracts/` / `quickstart.md` |

### 测试先行

- T003/T005/T008/T010/T012 均先写失败用例再写实现，失败信息保留在各阶段提交里。
- `parse_links` 初版用 `char` 索引在中文文本上触发 char-boundary panic，改写为 `text.as_bytes()`
  字节索引（`character.len_utf8()` 推进）后转绿；`task_dialog.rs` 的 `ui_task_dialog` 因在
  `egui::Window` 闭包内同时持 `self` 与 `self.task_detail.as_mut()` 触发 E0499，改为闭包外先取
  `task_id`、闭包内再借 `task_detail` 后通过。
- 当前规模：lib 71 条 + 集成 97 条 = 168 条，全部通过。

### 门禁（T014）

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS（含 4 处 `collapsible_if` 折叠为 `let_chains`） |
| `cargo test --all-targets --all-features` | PASS（lib 71 + 集成 97） |
| `cargo machete` | PASS，无未使用依赖 |
| `cargo audit` | PASS（5 条 allowed warnings，与 main 一致） |
| `git diff main -- Cargo.toml` | 013 无新增；差异来自 012（`egui_extras`/`image`/`rfd`/`rusqlite-backup`） |
| `git grep 'Color32::' -- src` | 仅 `src/app/theme.rs`（009 令牌文件） |
| `git grep '#\[(allow\|expect)(' -- src` | 无命中，无 lint 抑制 |
| 裸 `TODO` / `FIXME` | 源码无命中 |

### GUI 截图（T013）

- **本会话未执行**：T013 的隔离 GUI 截图验证（任务内容编辑、备注两栏图片/链接、音视频打开按钮）
  尚未进行，留待后续单独会话用与 012 相同的临时 `NGY_PREVIEW` 入口完成。代码门禁已全部转绿，
  功能由 lib 测试覆盖（`task_detail_loads_description_and_media` / `note_shows_source_task` 等）。
