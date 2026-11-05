# Implementation Plan: 用图标替换操作按钮

**Branch**: `010-icon-buttons` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

## Summary

新增一个自绘的 `IconButton` 控件，用简单线条形状替换界面上所有可独立表达的操作按钮；每个图标保留与原文一致的悬停提示，颜色沿用 009 的语义令牌。图标形状、提示文案与语义级别的映射集中在一处，替换只改按钮调用行，不触碰业务路径、持久化数据与依赖。

## Technical Context

**Language/Version**: Rust 1.97（edition 2024，`rust-toolchain.toml` 固定）

**Primary Dependencies**: `eframe` / `egui` 0.37（`egui::Widget` 与 `Painter` 自绘）；不新增依赖

**Storage**: 无变更；`settings.theme` 与全部业务表保持原样

**Testing**: `cargo test --all-targets --all-features`；新增 `src/app/tests/icons.rs` 覆盖映射表、两套主题渲染与点击行为不变；GUI 截图做人工排版确认

**Target Platform**: Windows 10/11 x64 桌面（首要目标）

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 纯外观替换，不新增热路径；沿用宪法 V 的桌面预算（启动 p95 < 1s、空闲 CPU < 1%、常驻内存 < 250MB），本次无性能相关代码变更

**Constraints**: 完全离线、不引入图标库/字体/图片/网络资源；单文件 ≤ 700 行；每个图标必须有中文悬停提示；命中区域不小于原按钮

**Scale/Scope**: 约 13 种形状覆盖 9 个界面文件中的全部操作按钮；两套主题共用同一形状集

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 新文件 `src/app/icons.rs` < 250 行；`src/app/main_view.rs` 等只替换调用行，行数基本持平；所有文件 < 600 | PASS |
| II. 质量与可维护性 | 替换后删除 009 中不再有消费者的 `theme::danger` / `theme::primary_button`；无死代码、无裸 TODO | PASS |
| III. 测试先行 | 图标映射与渲染用例先写并观察失败，再实现控件与替换 | PASS |
| IV. 接触面一致性 | 不新增对外 API；`IconButton` 为界面内部控件，构建器命名一致 | PASS |
| V. 性能与效率 | 每帧多画几条线段，无热路径变更；不触发前后测量要求 | PASS |
| VI. 简单性 | 用框架已有 `Widget` / `Painter`，不引入依赖；13 种形状覆盖全部按钮，不做可配置图标系统 | PASS |
| VII. 可观测性 | 不新增日志；点击仍走原有日志路径，错误只记一次 | PASS |
| VIII. 安全 | 无外部输入、无字符串拼接、无资源加载路径 | PASS |

**Phase 1 复核**：八项结论不变；映射表集中在 `contracts/icon-buttons.md`，实现只消费该表。

## Project Structure

### Documentation (this feature)

```text
specs/010-icon-buttons/
├── spec.md                    # /speckit.specify
├── checklists/requirements.md # /speckit.specify
├── plan.md                    # 本文件
├── research.md                # Phase 0：呈现方式、字形风险、命中区域
├── data-model.md              # Phase 1：Icon / IconButton 与不变量
├── contracts/icon-buttons.md  # Phase 1：逐按钮映射（唯一权威）
├── quickstart.md              # Phase 1：验证步骤
└── tasks.md                   # /speckit.tasks（尚未生成）
```

### Source Code (repository root)

```text
src/app/
├── icons.rs            # 新增：Icon 枚举、IconButton 控件、形状绘制
├── theme.rs            # 删除不再有消费者的 danger / primary_button
├── status_bar.rs       # 统计、知道了 → 图标
├── main_view.rs        # 结构树／任务行／新建弹窗按钮 → 图标
├── today_view.rs       # 完成／跳过／撤销 → 图标
├── popup.rs            # 查看今日清单 → 图标
├── rename_dialog.rs    # 保存／取消 → 图标
├── frequency_dialog.rs # 保存／取消 → 图标
├── estimate_dialog.rs  # 保存／取消 → 图标
├── delete_dialog.rs    # 删除／取消 → 图标
├── reset_dialog.rs     # 重置／取消 → 图标
├── stats_dialog.rs     # 关闭 → 图标
└── tests/
    ├── tests.rs        # 注册 mod icons
    └── icons.rs        # 新增：映射表、渲染与点击行为用例
```

**Structure Decision**: 单 crate 不变。图标形状与语义集中在新文件，各视图只把一行按钮调用换成一行 `ui.add(IconButton::new(...))`；009 的配色令牌与主题机制继续作为唯一颜色来源。

## Complexity Tracking

宪法检查八项全部 PASS，无需要论证的例外。

### 设计取舍

| 选择 | 理由 |
| --- | --- |
| 自绘线条形状而非 Unicode 符号 | 规避内置字体缺字导致「豆腐块」，且完全离线 |
| 13 种形状复用给多个按钮 | 形状少、易辨认，靠悬停文案区分语义；避免过度设计 |
| 固定 34×26 命中区域 | 不小于被替换的 `small_button`，任务行列宽可预期 |
| 颜色只取 `text_color()` / `error_fg_color()` | 沿用 009 语义令牌，切换主题自动跟随，不新增颜色字面量 |
| 删除 009 的 `danger` / `primary_button` | 替换后无消费者，按宪法 II 清除而非留着 |
| 主题下拉与文字信息不图标化 | 它们承载信息而非单一动作，图标化会丢失内容 |
