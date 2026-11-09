# 013 实施计划

## 复杂度追踪

- 新增实体：`Task` 增加 `description` 字段；新增 `TaskMedia`（`kind` / `file_name` / `original_name`
  / `byte_size` / `created_at`）与 `TaskMediaKind`（Image / Audio / Video）。
- 新增数据表 `task_content_media`；`tasks` 增加 `description` 列。schema 版本 3 → 4。
- 新增模块：`attachment.rs` 增加 `MediaStore`（与现有 `AttachmentStore` 并列，复用路径安全与落盘逻辑）。
- 改动文件：`model.rs`、`store/task.rs`（含 `task(id)` 获取器）、`store/schema.rs`、`attachment.rs`、
  `app/main_view.rs`（内容编辑入口）、新增 `app/task_dialog.rs`（任务内容编辑）、
  `app/note_dialog.rs`（左右两栏）、`app/describe.rs` 或新增 `render` 助手（富内容渲染）。

## 关键决策

- **内容存为纯文本 + 媒体附件**：文本上限 4000 字（`validate_description` 复用 `validate_note` 的
  控制字符规则但放大上限），不引入 HTML/富文本标记语言，避免注入面与解析依赖。链接在展示时按
  `http(s)://` 自动识别，渲染为可点击 `egui::Hyperlink`。
- **音频/视频交给操作系统默认程序**：`std::process::Command` 按平台调用 `open` / `xdg-open` /
  `cmd /c start`，不引入 `rodio`/`ffmpeg` 等解码依赖，符合原则 VI（简单性、YAGNI）。
- **媒体落盘复用附件目录隔离**：`MediaStore` 根目录为 `<数据目录>/task_content/<任务 ID>/`，与现有
  `attachments/<行动 ID>/` 对称；图片走解码 + 像素上限校验，音视频只做扩展名白名单 + 大小上限。
- **来源查看落在备注弹窗两栏**：不新增独立「任务详情」页，复用既有备注弹窗，左栏只读展示来源任务，
  右栏保持现有备注行为，最小化新交互面。

## 宪法复核（Constitution Check）

- I 文件规模：新增的 `task_dialog.rs`、`render` 助手均控制 < 600 行；`attachment.rs` 追加 `MediaStore`
  后仍 < 400 行；`model.rs`/`task.rs` 增量小。
- II 质量：无死代码、无注释代码、无裸 TODO；错误文案统一「哪个输入 / 期望 / 修正」。
- III 测试先行：内容校验、`task(id)`、描述读写、媒体增删、迁移、富内容渲染均有先红后绿用例。
- IV 接触面：媒体校验与描述校验错误文案沿用既有 `DomainError`/`AttachmentError` 风格。
- V 性能：媒体只在展示时按需加载缩略图（图片用既有 image loader），不预解码音视频；无热路径变更。
- VI 简单性：不新增 trait/开关；`MediaStore` 与 `AttachmentStore` 共享私有助手，三行相似代码优先于
  过早抽象。
- VII 可观测性：媒体落盘、描述写入带 `op` / `result` / `elapsed_ms`。
- VIII 安全：媒体文件名受单段校验（不逃出根目录），文本链接经 `egui::Hyperlink` 只打开 http(s)，
  不执行任意协议；SQL 全参数绑定。

## 依赖

- 复用既有 `rfd`（选文件）、`image`（图片解码）、`egui_extras`（图片加载）。不新增依赖。
