# Implementation Plan: 任务行布局与误触防护

**Branch**: `006-task-row-actions` | **Date**: 2026-11-04 | **Spec**: [spec.md](./spec.md)

## Summary

三项都落在界面层，存储层零改动：`reset_review_progress` 已经存在，只是被直接调用。

1. 新增 `src/app/reset_dialog.rs` 承载重置确认，与 `rename_dialog` / `delete_dialog` 对称
2. 任务行的频率标签改为可点击控件，去掉独立的「频率」按钮
3. 删除按钮前加一处间距

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 无新增

**Storage**: 不涉及；沿用 `reset_review_progress`

**Testing**: `src/app/tests.rs` 补三条：确认后归零、取消后不变、任务已删除时弹窗保留

**Performance Goals**: 无新增端点，沿用宪法第 V 条预算

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | `main_view.rs` 当前 451 行，本次净减少（去掉频率按钮、加一处 spacing），约 449 行；弹窗独立成模块，约 80 行 | PASS |
| II. 质量与可维护性 | 三个弹窗共用同一套「打开－确认－关闭」形态，但不提前抽象成通用确认组件，按原则 VI 先忍 | PASS |
| III. 测试先行 | 先写 `request_reset` / `confirm_reset` / `cancel_reset` 的失败用例，再实现 | PASS |
| IV. 接触面一致性 | 确认框按钮沿用「重置」「取消」；错误沿用 `DomainError` 文案 | PASS |
| V. 性能与效率 | 无新增 IO；当前级别从内存里的 `task_progress` 取，不额外查库 | PASS |
| VI. 简单性 | 不引入撤销栈；不做通用确认组件；间距只加在删除按钮前 | PASS |
| VII. 可观测性 | 重置复用既有 `OpTrace` | PASS |
| VIII. 安全基线 | 只读写本地库，参数绑定 | PASS |

## Project Structure

```text
src/
├── app.rs                # reset_target 状态
├── app/reset_dialog.rs   # 重置确认弹窗（新）
├── app/main_view.rs      # 频率标签入口、删除按钮间距
└── app/tests.rs          # 三条重置用例
specs/006-task-row-actions/
├── spec.md、plan.md、tasks.md
```

**Structure Decision**: 不新建 `confirm_dialog.rs` 通用组件；三个弹窗各自一个模块，形态重复第二次，等第三次出现再考虑提取。

## Complexity Tracking

| 选择 | 理由 |
| --- | --- |
| 不抽象通用确认框 | 删除框要展示影响范围、重置框要展示当前级别，文案差异大于共性 |
| 频率标签用 `selectable_label(false, …)` | 与现有树上的可点击文本同一套视觉，不引入新控件样式 |
| 重置不清 `actions` | 历史记录是审计依据，重置只影响未来调度 |

## Phase 完成情况

- Phase 0: 无新研究项
- Phase 1: 本文件与 [tasks.md](./tasks.md)
- Phase 2: 待实现
