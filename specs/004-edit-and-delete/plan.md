# Implementation Plan: 重命名与任务删除的操作入口

**Branch**: `004-edit-and-delete` | **Date**: 2026-11-04 | **Spec**: [spec.md](./spec.md)

## Summary

把存储层已有的四个方法接到界面：目标、分类、任务各加「改名」入口，任务行加「删除」入口。重命名用一个共用弹窗（按目标/分类/任务区分校验），任务删除复用目标与分类已有的确认弹窗机制，并在确认前给出「将作废 N 条未完成行动」。

技术路径：`App` 增加重命名状态（目标对象 + 预填名称），`PendingDelete` 增加任务变体，`Store` 增加一个只读计数查询供确认弹窗使用。表结构与既有查询不变。

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 无新增

**Storage**: SQLite，表结构不变；新增一个 `COUNT(*)` 只读查询

**Testing**: `cargo test --all-targets --all-features`；存储层计数查询补集成用例，界面交互靠手工验收（沿用 001 的 D 决策）

**Performance Goals**: 沿用宪法第 V 条桌面预算（1.18.0）；本 feature 不新增需要测量的端点

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | `main_view.rs` 当前约 500 行，本次预计 +120 行，仍低于 600 行预警线；若越线则把三个弹窗拆到独立模块 | PASS |
| II. 质量与可维护性 | 新增状态与方法补契约注释；不新增 lint 抑制 | PASS |
| III. 测试先行 | 计数查询先写失败用例；界面部分无自动化框架，手工验收补 | PASS |
| IV. 接触面一致性 | 重命名弹窗与新建弹窗的布局、按钮文案、错误位置保持一致 | PASS |
| V. 性能与效率 | 计数查询走 `task_id` 索引路径，单条语句 | PASS |
| VI. 简单性 | 三类对象共用一个弹窗与一个状态，不写三套；不引入通用的「实体编辑器」抽象 | PASS |
| VII. 可观测性 | 复用 `OpTrace` 记录新查询；删除与改名沿用既有日志 | PASS |
| VIII. 安全基线 | 名称经 `validate_name` 校验，SQL 参数绑定 | PASS |

## Project Structure

```text
src/
├── app.rs                # RenameTarget 枚举、draft_rename_name、PendingDelete 增加 Task
├── app/main_view.rs      # 三类对象的「改名」「删除」入口 + 重命名弹窗
└── store/task.rs         # task_open_actions（未完成行动计数）
tests/
└── store_test.rs         # 计数查询用例
specs/004-edit-and-delete/
├── spec.md、plan.md、contracts/ui-and-store.md、quickstart.md、tasks.md
```

**Structure Decision**: 重命名状态放 `App`（跨行共享），弹窗渲染放 `main_view`，与既有新建弹窗同处。不新建模块，除非 `main_view.rs` 越过 600 行。

## Complexity Tracking

无破坏性变更、无宪法违规。两点说明：

| 选择 | 理由 |
| --- | --- |
| 三类对象共用一个重命名弹窗 | 三套弹窗的差异只有校验函数与提交目标，共用后新增第四个可编辑对象只需加一个枚举分支 |
| 任务删除复用 `PendingDelete` 而不是新建机制 | 目标与分类已有「先算影响范围再确认」的完整链路，任务只是多一个变体与一个计数查询 |
| 新增计数查询而不是在 UI 层数 `self.actions` | `self.actions` 只有今天的行动，未完成行动还包括历史顺延记录，数它会低估影响范围 |

## Phase 完成情况

- Phase 0: 无新研究项；四个存储方法与删除确认机制已存在
- Phase 1: [contracts/ui-and-store.md](./contracts/ui-and-store.md)、[quickstart.md](./quickstart.md)
- Phase 2: 待 `/speckit.tasks`
