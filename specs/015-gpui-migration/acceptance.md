# 015 gpui-kit 迁移 · 验收记录

**分支**：`015-gpui-migration`（基于 `main`，未改动 `main`）
**作者**：单开发者自我复核（宪法「单开发者仓库自我证明」）
**日期**：2026-11-13

## 一、迁移范围与结果

将原有 `egui` + `eframe` 界面整体迁移到 `gpui-kit`（gpui 0.9）。`Cargo.toml` 已无 `egui`/`eframe`，`src/` 无任何 egui 残留；数据层（`store`/`model`/`config`/`common`）零改动。

| 模块 | 状态 | 说明 |
| --- | --- | --- |
| 结构树（目标/分类/任务） | ✅ | `app.rs` 渲染，改名/删除/频率/重置/备注按钮 |
| 三页签（任务/今日/历史） | ✅ | today_view / history_view / task_editor 各自 `tests.rs` 验收 |
| 今日页 | ✅ | 生成/结算/跳过，计数回到结构树 |
| 历史页 | ✅ | 分页/结算/撤销，就地更新内存分页 |
| 任务内容编辑器 | ✅ | 名称/描述/预计耗时/频率编辑，写回 store |
| 备注对话框 | ✅ | 左右两栏（来源任务/当前备注），写回 action.note |
| 富文本查看 | ✅ | 解析 `**粗**`、`*斜*`、`- 列表` |
| 重命名对话框 | ✅ | 目标/分类改名，错误可重试 |
| 删除对话框 | ✅ | 影响范围预览 + 级联删除确认 |
| 频率对话框 | ✅ | 改频率，影响范围预览 |
| 统计对话框 | ✅ | 7/30/90 天复习统计 |
| 存储对话框 | ✅ | 数据目录占用递归累加 |
| 重置进度对话框 | ✅ | 补 plan 第 16 行遗漏的 `reset_dialog.rs`；确认归零，任务不存在保留并提示 |
| 工具栏/菜单栏/状态栏 | ✅ | 每日上限步进（越界拒）、主题切换（System/Light/Dark 持久化并应用）、统计/存储入口；启动应用已存主题 |
| 测试迁移（T016） | ✅ | 原 `src/app/tests/*` egui headless 目录随迁移早期删除；行为由现 gpui-kit 测试接管 |

## 二、门禁（六项 CI 本地全部转绿）

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ |
| `cargo test --all-targets --all-features` | ✅（lib 53 + 集成 100+ 全绿） |
| `cargo machete`（未用依赖） | ✅ 无 |
| `cargo audit`（漏洞） | ⚠️ 仅 `zbus`/`zvariant` 传递依赖 `unmaintained` 与 `async-executor` `yanked`（warning 级，CI `cargo audit` 不阻断）；属 gpui-kit 的 Linux D-Bus 依赖，非 015 直接引入，未新增抑制 |
| 治理作业（文件行数/死代码抑制） | ✅ `app.rs` 679 行（< 700 上限），全仓零 `#[allow(dead_code)]` |

## 三、宪法复核（I–VIII）

- **I 文件规模**：最大源文件 `app.rs` 679 行，其余均 < 700；拆分出 `toolbar.rs`/`dialogs.rs`。
- **II 质量/可维护性**：移除惰性 `spawn_ticker` 与死字段 `ticks`/`bootstrap`/`selected_task`（顺带移除未用 `tokio` 依赖）；无 `#[allow(dead_code)]`，无注释代码，无 TODO。
- **III 测试先行**：每个对话框/页签按 TDD 先写测试后实现。
- **IV 对外接触面**：`MinActionApp::new` 等库 API 命名统一；错误走 `Result<DomainError>`。
- **V 性能**：无性能敏感变更。
- **VI 简单性**：无当前消费者的配置/开关/抽象均已删除。
- **VII 可观测性**：沿用既有 `tracing` 结构化日志，无 stdout 污染。
- **VIII 安全**：输入在边界校验（如每日上限越界拒），无拼接注入，无机密入库。

## 四、GUI 截图验证（T018，代理于锁屏桌面执行）

执行环境为**锁屏中的 Windows 桌面**：`CopyFromScreen` 只能拍到锁屏界面，且锁屏下
`SendInput` 点击进不了应用桌面。改用 `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`
直接抓 gpui 窗口内容成功；主题切换不经点击，直接改隔离库 `settings.theme` 后重启应用。

流程：隔离 `%LOCALAPPDATA%=D:\temp\ngy-gui-test` → `cargo build` → 启动
`target/debug/ngy-min-action.exe` → `sqlite3` 预置 2 目标/3 分类/4 任务与
`settings.theme`（light / dark 各一次）→ 每次启动后 `PrintWindow` 抓窗口。

### 已验证 ✅

| 项 | 结果 |
| --- | --- |
| 应用启动与渲染 | gpui-kit 窗口正常创建并渲染，无崩溃 |
| 结构树与计数 | 健康(3)/锻炼(2)/饮食(1)/学习(1)，与种子数据一致，含 重命名/删除 按钮 |
| 任务页签 | 所选分类「锻炼」下展示 晨跑/力量训练 |
| 主题持久化与切换 | `settings.theme`=light/dark 重启后均正确应用（深色按钮相应高亮） |
| 深色主题外观 | 全暗背景，文字/按钮/高亮一致，无异常 |

截图：`D:\temp\shot_light.png`、`D:\temp\shot_dark.png`（1550×1133，未入库）。

### 发现的外观问题 ⚠️

**浅色主题下部分元素为硬编码深色，不随主题切换**：应用头部「最小行动」条、结构树选中行
（`structure_tree.rs` 的 `rgb(0x2a_2a_2e)`）、任务列表行、「任务」页签胶囊、主题按钮胶囊。
深色主题下无此问题（一切本就深色）。属样式打磨项，不阻塞迁移验收。

### 未覆盖（锁屏无法点击，需解锁后人工补验）

「今日待办」「历史待办」页签、工具栏每日上限步进、统计/存储/重置三个对话框的截图。
