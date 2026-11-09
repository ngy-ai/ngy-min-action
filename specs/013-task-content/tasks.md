# 013 任务清单

## Phase 1: 基础（数据模型、迁移、存储）

- [ ] T001 在 `spec.md` / `plan.md` / `tasks.md` 记录需求、宪法复核与复杂度追踪
- [ ] T002 模型：`Task` 增加 `description: Option<String>`，新增 `TaskMedia` / `TaskMediaKind` /
      `TaskContentMediaId`，`TaskDraft` 增加 `description`；新增 `MAX_TASK_DESCRIPTION` 与
      `validate_description`
- [ ] T003 先写失败用例：内容校验、按 `id` 取任务、描述读写、媒体增删
- [ ] T004 schema 3 → 4：`tasks.description` + `task_content_media` 表，2→3 之外的旧库清空重建，
      保留业务数据
- [ ] T005 实现 `task(id)`、`task_description` / `set_task_description`、
      `add_task_media` / `task_media` / `delete_task_media`（先红后绿）
- [ ] T006 `attachment.rs` 增加 `MediaStore`：图片走解码校验，音视频走扩展名 + 大小白名单；
      复制落盘与单段文件名校验

## Phase 2: 编辑入口

- [ ] T007 新增 `app/task_dialog.rs`：任务内容编辑——多行 `TextEdit` + 媒体「添加 / 删除」
      （`rfd` 选文件），从任务区打开
- [ ] T008 失败用例：内容保存/清空、媒体增删、非法文件被拒；保存后界面刷新

## Phase 3: 富内容渲染

- [ ] T009 新增渲染助手：文本自动识别 `http(s)://` 为可点击链接；媒体块按类型——图片内联、
      音视频「打开」按钮（OS 默认程序）
- [ ] T010 失败用例：链接解析、媒体类型映射、空内容

## Phase 4: 备注弹窗两栏

- [ ] T011 `note_dialog.rs` 改左右两栏：左 = 来源任务（标题 + 渲染内容），右 = 当前备注；
      只读态两栏均只读
- [ ] T012 失败用例：两栏结构渲染、来源任务标题/内容显示、只读态

## Phase 5: 验证与交付

- [ ] T013 隔离 `%LOCALAPPDATA%` 起 GUI，深浅截图：任务内容编辑、备注两栏（含图片/链接）、
      音视频打开按钮
- [ ] T014 门禁：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、
      `cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`；复核
      `git diff main -- Cargo.toml` 与颜色字面量
- [ ] T015 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者
      仓库自我证明）

## Notes

- `MediaStore` 与既有 `AttachmentStore` 共享 `is_single_component` / `copy_new` / `generated_name`
  私有助手，避免重复。
- 迁移（`storage.rs`）按整目录复制数据目录，`task_content/` 随附件一起移动，无需特殊处理。
