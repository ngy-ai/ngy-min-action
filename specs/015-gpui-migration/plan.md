# 015 实施计划

## 复杂度追踪

- 架构级变更：`egui` 的「单 `MinActionApp` 结构 + `Ui` 借用」模式 → `gpui-kit` 的
  「`View` / `Entity` / `Model` + `cx` 上下文 + 事件」模式。根视图持有 UI 状态，`Store` 以
  `Arc<Store>` 注入；子视图（树、页签、对话框）通过 `WeakEntity<RootView>` 回调，或经
  `EventEmitter` 通知根视图刷新。
- 新增/改写模块（均 < 700 行，超限再拆）：
  - `main.rs` / `lib.rs`：gpui-kit 入口（`application().run` + `init` + `open_window`）。
  - `app.rs`：根 `View`（`MinActionApp`）持有 `Arc<Store>` + UI 状态 + `reload()`，原 egui 方法逻辑平移。
  - `app/main_view.rs` → 结构树（左栏）`View`。
  - `app/tabs.rs`（新）：三页签容器。
  - `app/today_view.rs` / `app/history_view.rs`：gpui 版。
  - `app/task_dialog.rs` / `note_dialog.rs` / `rename_dialog.rs` / `delete_dialog.rs` /
    `frequency_dialog.rs` / `estimate_dialog.rs` / `reset_dialog.rs` / `storage_dialog.rs` /
    `stats_dialog.rs` / `structure_menu.rs` / `popup.rs` / `rich_content.rs` / `icons.rs` /
    `theme.rs` / `describe.rs` / `status_bar.rs` / `toolbar.rs` / `menu_bar.rs`：逐个 gpui 版改写。
  - 删除：`egui_extras` 相关 `FileLoader` 用法（`rich_content.rs` 改用 gpui-kit `Img`/`img()`）。

## 关键决策

- **数据层零改动**：`Store` / `Model` / `AttachmentStore` / `MediaStore` 原样复用。根视图持有
  `Arc<Store>`（Store 已自管理状态，无需 gpui `Model` 订阅机制），调用方式与 egui 版一致。
- **组件映射表（egui → gpui-kit）**：基础元素均来自 gpui-kit，高级组件在 `gpui_kit::component` 下。
  若某高级组件实际 API 与预期不符，一律用基础 `Div` / `List` / `Img` / `Text` / `Button` 自建，
  不阻塞迁移（T002 确认实际模块名）。

  | egui | gpui-kit |
  | --- | --- |
  | `ui.heading("…")` | `Heading` 或 `div().text("…").text_xl()`（component 文本） |
  | `ui.label` / `ui.weak` | `Text` / `div().child("…")` |
  | `TextEdit::singleline` / `multiline` | `gpui_kit::component::input` 的 `TextInput` |
  | `ui.button` / `icons::add` | `Button`（component::button），图标用 kit 内置 Lucide |
  | `ui.selectable_label`（页签） | `Tab` 组件（component）或高亮 `Button` |
  | `ScrollArea` | `uniform_list` / 可滚动 `div` |
  | `egui::Image` | `Img` / `img(Path)` |
  | `egui::Hyperlink` | 链接文本 + 打开命令（沿用 013 的 OS 默认程序） |
  | `egui::Window`（对话框） | `ManagedView` Modal/Popover（component）或自建浮层 `View` |
  | `ui.columns(2)`（两栏） | `div().flex().row().child(...).child(...)` |

### T002 调研结论（gpui-component 0.9.1 已确认）

经 docs.rs 核对，以下组件模块均存在，可直接使用（路径为 `gpui_kit::component::<模块>`）：

- `button`（`Button`）、`input`（文本输入）、`tab`（页签/`TabBar`）、`list`（`v_virtual_list` /
  `VirtualList` 虚拟列表）、`dialog`（模态弹窗，`WindowExt::dialog`/`sheet`）、`tree`（树）、
  `menu` / `native_menu`、`select` / `combobox`、`sidebar`、`status_bar`、`toolbar`、`text`（富文本）。
- 左右分栏用 `resizable` 模块的 `resizable_panel()` / `h_resizable` / `v_resizable`。
- 布局容器 `h_flex()` / `v_flex()`；初始化 `gpui_kit::init()`。
- 图片用 `gpui_kit` 根级 `Img` / `img(Path)`（`gpui_component` 无独立 img 模块）。
- headless 测试沿用 gpui/gpui-kit 的测试宏（类似 `#[gpui::test]`），构造 `AppContext` 渲染真实组件。

映射表据此为「可落地」而非「待确认」。

- **媒体**：保留 `image` crate（图片解码校验）与 `rfd`（选文件）；渲染用 gpui-kit `Img`/`img()`
  加载本地路径，不再依赖 `egui_extras` 的 `FileLoader`。
- **主题**：gpui-kit 内置主题系统，先用默认主题；`ThemeChoice`（System/Light/Dark）映射到 kit
  主题设置（T015 确认 API，必要时降级为固定主题）。
- **测试**：用 gpui-kit / gpui 的 headless 测试（类似 `#[gpui::test]`）构造 `AppContext` /
  `TestAppContext`，创建根视图，断言状态与渲染无 panic；沿用「直接调用视图/状态方法、不依赖鼠标」
  的验收风格（见 013 tests），避免 GUI 自动化环境依赖。

## 宪法复核（Constitution Check）

- **I 文件规模**：每个 gpui 视图文件 < 700 行；根视图协调逻辑若超 600 行预警线即拆分子视图。
- **II 质量**：无 lint 抑制；无死代码；对话框/输入的错误文案沿用既有 `DomainError` 风格。
- **III 测试先行**：每阶段先写 gpui-kit 失败测试（编译失败或断言失败），再写最少实现。
- **IV 接触面**：`Store` API 完全不变；UI 内部重构不新增对外接触面。
- **V 性能**：沿用宪法「桌面 GUI 派生项目」预算——启动到可交互 p95 < 1s；列表用 `uniform_list`
  虚拟滚动（200 任务规模）。gpui-kit 为 GPU 渲染，120fps 能力足够。
- **VI 简单性**：组件映射 1:1，不新增 trait/开关；无当前无消费者的抽象。
- **VII 可观测性**：沿用 `tracing` 结构化字段（`op` / `result` / `elapsed_ms`），错误只记一次。
- **VIII 安全**：媒体文件名单段校验、文本链接只开 `http(s)` 等沿用 013；新增依赖经 `cargo audit`。

## 依赖

- 移除：`eframe = "0.37"`、`egui_extras = { ..., features = ["image", "file"] }`（二者带入 `egui`）。
- 新增：`gpui-kit = "0.9"`（默认 feature 已含 `gpui-component` 与 `assets`/Lucide 图标；如需
  `gpui-fast` 后端待 T002 评估）。
- 保留：`image`（媒体校验）、`rfd`（选文件）、`tokio`、`rusqlite`、`chrono`、`anyhow`、
  `tracing`、`ngy-utils-tracing`、`caseless`；dev：`criterion`。
- `Cargo.lock` 由 `cargo` 命令更新，不手改。
