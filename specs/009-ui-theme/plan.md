# Implementation Plan: 界面美化与明暗主题

**Branch**: `009-ui-theme` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

## Summary

在界面层引入两套可切换的配色与统一的排版令牌，用户从顶栏在「跟随系统 / 浅色 / 深色」之间选择；选择写进现有 `settings` 表并在启动时直接应用。系统主题检测与两套样式切换交给 egui 0.37 自带的 `ThemePreference` / `RawInput::system_theme`，不引入新依赖，也不改业务表结构。视觉美化限于令牌化的间距、字号层级、语义色与操作分组，保持现有信息架构。

## Technical Context

**Language/Version**: Rust 1.97（edition 2024，`rust-toolchain.toml` 固定）

**Primary Dependencies**: `eframe` / `egui` 0.37（界面与主题）、`rusqlite` 0.41 bundled（设置持久化）、`tracing` 0.1（结构化日志）；不新增依赖

**Storage**: 单文件 SQLite，复用现有 `settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)` 表，新增 `theme` 键；不改任何业务表

**Testing**: `cargo test --all-targets --all-features`；`tests/theme_test.rs` 覆盖设置读写契约，`src/app/tests/theme.rs` 覆盖 headless 主题状态与数据不变，GUI 截图做人工排版确认

**Target Platform**: Windows 10/11 x64 桌面（首要目标）

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 主题切换只是样式选择，不新增热路径；沿用宪法 V 的桌面预算（启动 p95 < 1s、空闲 CPU < 1%、常驻内存 < 250MB），本次无性能相关代码变更

**Constraints**: 完全离线；单文件 ≤ 700 行；不引入图标库/字体/网络资源；偏好值在边界校验；首帧即应用主题

**Scale/Scope**: 单用户单机；≤ 200 个任务；两套主题覆盖主界面、结构树、今日清单与五个弹窗

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | `src/model.rs` 现 563 行，新增 `mod theme` 与 `pub use` 后仍 < 600；新增文件各 < 150 行 | PASS |
| II. 质量与可维护性 | 新公共 API 写契约注释；无死代码、无注释代码、无裸 TODO | PASS |
| III. 测试先行 | `theme_choice` / `set_theme_choice` 与主题状态先写失败用例，再写最少实现 | PASS |
| IV. 接触面一致性 | 命名对齐既有 `daily_cap()` / `set_daily_cap()`；非法值返回 `DomainError::InvalidValue`，未知持久化值按边界校验回退 | PASS |
| V. 性能与效率 | 无热路径变更；`ctx.set_theme` 只在用户切换或启动时调用，不进每帧循环；不触发前后测量要求 | PASS |
| VI. 简单性 | 复用 `settings` 表与 egui 内置 `ThemePreference`；不引入 `dark-light` 之类依赖；三态偏好是 spec 明确需求，不是预留开关 | PASS |
| VII. 可观测性 | 切换与读取沿用 `OpTrace` 的 `op` / `result` / `elapsed_ms`；错误只在边界记一次 | PASS |
| VIII. 安全基线 | 偏好值在存储边界解析校验，SQL 全参数绑定，无外部字符串拼接 | PASS |

**Phase 1 复核**：设计完成后八项结论不变；新增字段只在 `settings` 增加一个键，业务表零改动。

## Project Structure

### Documentation (this feature)

```text
specs/009-ui-theme/
├── spec.md              # /speckit.specify
├── checklists/
│   └── requirements.md  # /speckit.specify
├── plan.md              # 本文件
├── research.md          # Phase 0：主题检测、持久化、配色与验证方式
├── data-model.md        # Phase 1：偏好实体与设置键
├── contracts/
│   └── ui-theme.md      # Phase 1：界面契约
├── quickstart.md        # Phase 1：验证步骤
└── tasks.md             # /speckit.tasks（尚未生成）
```

### Source Code (repository root)

```text
src/
├── model.rs              # 模块根：新增 mod theme / pub use
├── model/
│   └── theme.rs          # 领域枚举 ThemeChoice 与字符串互转、边界校验
├── store.rs              # 模块根：新增 mod theme
├── store/
│   └── theme.rs          # settings 表中 theme 键的读写
├── app.rs                # 应用状态：theme 字段、切换入口、错误展示
├── app/
│   ├── theme.rs          # 配色/间距/字号令牌，两套 Style，对比度计算
│   ├── main_view.rs      # 任务行与空状态改用语义样式
│   ├── today_view.rs     # 今日清单对齐
│   ├── *_dialog.rs       # 五个弹窗统一标题/间距/操作分组
│   └── tests/
│       └── theme.rs      # headless 主题状态与数据不变用例
├── run.rs                # eframe 创建回调中应用首帧主题
└── main.rs

tests/
└── theme_test.rs         # 设置读写契约与非法值回退
```

**Structure Decision**: 单 crate 不变。领域枚举与设置读写各自成文件、按职责命名；界面令牌集中到 `src/app/theme.rs`，界面其余文件只消费令牌，不出现颜色字面量。新增测试镜像源码布局（`tests/theme_test.rs`、`src/app/tests/theme.rs`）。

## Complexity Tracking

宪法检查八项全部 PASS，无需要论证的例外。

### 桌面预算与测量

本次不改动热路径，不产生新的性能预算例外；验证沿用 `scripts/perf.ps1`，不重复测量。

### 设计取舍

| 选择 | 理由 |
| --- | --- |
| 用 egui 内置 `ThemePreference` | 框架已提供系统主题注入与两套样式切换；自研系统主题轮询需要 Win32 依赖且违背 VI |
| 偏好存 `settings` 表 | 已有 key-value 表与 `daily_cap` 的成熟读写模式，不新增配置文件或环境变量 |
| 两套 `Style` 而非逐控件改色 | 令牌集中定义后弹窗与视图自动继承，切换时不会残留旧色 |
| 用 `Visuals` 的语义色字段 | `warn_fg_color` / `error_fg_color` 由两套主题各自定义，界面不写死颜色值 |
| 对比度用令牌单元测试验证 | 所有控件颜色都来自令牌，逐令牌验证等价于逐控件验证，且可自动回归 |
