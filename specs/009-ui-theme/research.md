# Research: 界面美化与明暗主题

**Feature**: `009-ui-theme` | **Date**: 2026-11-05

本文件记录 Phase 0 的技术决策，依据为 `spec.md`、宪法与 `egui` 0.37 源码（本地 crates 源码），不依赖网络资料。

## D1: 系统明暗主题如何检测与跟随

**Decision**: 使用 egui 0.37 自带的 `RawInput::system_theme` 与 `Context::set_theme(ThemePreference::System)`。eframe 每帧把系统主题注入 `RawInput`，egui 的 `Options::theme()` 在 `System` 模式下用 `system_theme` 选择对应样式，系统主题变化会自动生效。

**Rationale**: 满足 FR-005（运行中跟随、2 秒内），不需要轮询、Win32 注册表读取或额外依赖；手动选浅色/深色时 `set_theme` 固定偏好，系统变化不再干扰。

**Alternatives considered**:
- 用 `dark-light` / `system-theme` 之类 crate 查询系统主题：新增依赖且仍需自己轮询，违背 VI
- 读 Windows 注册表 `AppsUseLightTheme`：需要 Win32/注册表依赖，且无法覆盖运行中变化，跨平台也失效
- 只做手动两态、取消「跟随系统」：与 spec 的 FR-004/FR-005 冲突

## D2: 偏好存哪里

**Decision**: 复用现有 `settings(key, value)` 表，新键 `theme`，取值 `system` / `light` / `dark`；读取时解析，任何未知值或空值回退 `system`。

**Rationale**: FR-013 要求不改业务表结构，`settings` 已承载 `daily_cap`，读写模式（参数绑定 + `ON CONFLICT DO UPDATE` + `OpTrace`）可直接复用。

**Alternatives considered**:
- 新增 `theme_preferences` 表：无必要，且属于「为以后可能用到而预建结构」
- 写 `.env` / 配置文件：与「完全离线、不新增配置文件」假设冲突，且宪法 VIII 不鼓励本地配置入库
- 只存内存：不满足 FR-003 的重启恢复

## D3: 两套配色与视觉令牌怎么组织

**Decision**: 新建 `src/app/theme.rs`，用 `Context::set_style_of(Theme::Dark | Theme::Light, style)` 分别注册两套 `Style`。每套基于 `Theme::default_style()`，只覆盖 `visuals` 的语义字段（`warn_fg_color`、`error_fg_color`、`hyperlink_color`、面板/窗口填充、弱文本色）与 `spacing`（`item_spacing`、`button_padding`、`indent`）、`text_styles` 的字号层级。界面代码只使用 `ui.visuals()` 的语义色与 `ui.weak()` / `ui.heading()`。

**Rationale**: 一次注册后所有视图与弹窗自动继承，切换主题不会残留旧色（FR-002、FR-007）；语义色由两套主题各自定义，浅深都可读（FR-006、FR-010）。

**Alternatives considered**:
- 逐控件判断当前主题再挑颜色：逻辑散落、易漏，且必然产生颜色字面量
- `ctx.set_visuals(Visuals::dark()/light())` 只改当前主题：切换后另一套没有我们的语义色，弹窗会闪回默认色
- 引入完整设计系统/主题 crate：超出本 feature 范围，违背 VI

## D4: 首帧不闪烁

**Decision**: 在 `eframe::run_native` 的 `CreationContext` 回调里，先注册两套 `Style` 再按已保存偏好调用 `ctx.set_theme(...)`，然后返回 `MinActionApp`。偏好在 `MinActionApp::new` 已读进内存。

**Rationale**: 回调早于首帧渲染，满足 FR-012；不需要在 `App::ui` 首帧打补丁。

**Alternatives considered**:
- 在 `App::ui` 第一帧设置：首帧已按默认主题绘制，会出现可见闪烁
- 在 `main` 里读系统主题并硬编码：与 D1 的跟随系统冲突

## D5: 对比度如何验证

**Decision**: 在 `src/app/theme.rs` 提供 WCAG 相对亮度与对比度计算（供测试使用），`src/app/tests/theme.rs` 对两套令牌的正文/弱化文字/语义色逐项断言 ≥ 4.5:1、大字号 ≥ 3:1；再用隔离数据目录的 GUI 截图做人工排版确认。

**Rationale**: 所有控件颜色都来自令牌，逐令牌验证等价于逐控件验证，且能自动回归（SC-001、SC-004）。

**Alternatives considered**:
- 截图后逐像素取色计算：依赖屏幕缩放与抗锯齿，结果不稳定
- 只靠人工观察：无法进 CI 回归

## D6: 「美化」的可实施范围

**Decision**: 统一四类元素：① 面板内边距与控件间距；② 标题/正文/弱化文字的字号层级；③ 主要、次要、危险操作的视觉区分（危险操作沿用二次确认）；④ 空状态与错误提示统一样式。任务行列对齐与长名称截断规则保持不变。

**Rationale**: 满足 FR-008/FR-009/FR-010，同时不调整信息架构与主布局（Assumptions）。

**Alternatives considered**:
- 重排主界面成侧边栏 + 内容区：信息架构变更，超出 spec 范围
- 引入图标与自定义字体：违反 FR-014 的离线与不引入资源约束

## D7: 是否需要新增依赖

**Decision**: 不需要。egui 0.37 已提供主题偏好、系统主题、两套样式与语义色字段；SQLite 复用 `rusqlite`。

**Rationale**: 宪法 VI 要求直接使用依赖已有能力，避免「依赖先进、用处后到」。
