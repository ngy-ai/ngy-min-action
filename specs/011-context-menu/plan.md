# Implementation Plan: 结构树改用右键菜单

**Branch**: `011-context-menu` | **Date**: 2026-11-06 | **Spec**: [spec.md](./spec.md)

> **文档裁剪说明**：本改动范围小、无新持久化实体、无外部契约，Phase 0 研究与 Phase 1 设计结论合并到本文件，不单独生成 `research.md` / `data-model.md` / `contracts/` / `quickstart.md`；验证步骤并入 `tasks.md` 的验收记录。宪法未要求固定数量的设计文档。

## Summary

把结构树目标／分类行的两个行内图标按钮换成名称控件的右键菜单：菜单项「改名」「删除」集中定义，动作沿用原有的重命名弹窗与删除二次确认；名称控件保留左键选中并增加悬停提示。同时清掉 010 图标映射中失去消费者的两个 `ButtonId` 变体。

## Phase 0 / Phase 1 结论

| 决策 | 结论 |
| --- | --- |
| 菜单 API | egui 0.37 的 `Response::context_menu`（不消耗 `Response`，可继续 `.clicked()`）；菜单项点击后用 `Ui::close()` 关闭 |
| 触发区域 | 仅名称 `selectable_label`；行内空白、展开箭头不触发 |
| 菜单项形式 | 文字标签；「删除」用 `error_fg_color()` 上色。菜单不是 010 定义的工具条按钮，故不图标化 |
| 动作传递 | 菜单闭包把动作写入 `ui_main_view` 已有的局部状态变量（`rename` / `drop_goal` / `drop_category`），沿用原有统一处理路径 |
| 可发现性 | 名称控件 `.on_hover_text("左键选中，右键菜单")` |
| 可测性 | 菜单项抽成 `MENU_ITEMS` 常量表 + `show(ui)` 绘制函数；headless 用例断言表内容与两套主题渲染，避免依赖真实右键事件注入 |
| 死代码 | `ButtonId::DeleteGoal` / `DeleteCategory` 失去消费者，从枚举、`ALL` 与 `spec()` 一并删除；`Rename` 仍被任务行使用 |
| 依赖与数据 | `Cargo.toml` 不变；不改业务表、`settings` 键与调度逻辑 |

## Technical Context

**Language/Version**: Rust 1.97（edition 2024）

**Primary Dependencies**: `eframe` / `egui` 0.37；不新增依赖

**Storage**: 无变更

**Testing**: `cargo test --all-targets --all-features`；新增 `src/app/tests/context_menu.rs`；GUI 截图人工确认

**Target Platform**: Windows 10/11 x64 桌面

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 无热路径变更；沿用宪法 V 的桌面预算

**Constraints**: 单文件 ≤ 600 行预警线；无 lint 抑制；颜色只取语义令牌；保持完全离线

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 新文件 `src/app/structure_menu.rs` < 80 行；`main_view.rs` 行数下降 | PASS |
| II. 质量与可维护性 | 删除无消费者的 `ButtonId` 变体；无死代码、无裸 TODO | PASS |
| III. 测试先行 | 菜单项与渲染用例先失败再实现 | PASS |
| IV. 接触面一致性 | 不新增对外 API；菜单为界面内部控件 | PASS |
| V. 性能与效率 | 菜单只在右键时渲染，无热路径变更 | PASS |
| VI. 简单性 | 复用框架 `context_menu` 与既有状态变量，不新建菜单框架 | PASS |
| VII. 可观测性 | 不新增日志；动作仍走原有日志路径 | PASS |
| VIII. 安全 | 无外部输入、无资源加载 | PASS |

## Project Structure

```text
specs/011-context-menu/
├── spec.md
├── checklists/requirements.md
├── plan.md
└── tasks.md

src/app/
├── structure_menu.rs   # 新增：MENU_ITEMS、StructureAction、show(ui)
├── main_view.rs        # 目标／分类行：去掉两个图标按钮，名称控件接 context_menu
├── icons.rs            # 删除 DeleteGoal / DeleteCategory 变体及映射
└── tests/
    ├── tests.rs        # 注册 mod context_menu
    └── context_menu.rs # 新增：菜单项定义与双主题渲染用例
```

## Complexity Tracking

宪法检查八项全部 PASS，无例外。唯一偏差是设计文档裁剪（见开头说明），不产生治理例外。
