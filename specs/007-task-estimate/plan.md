# Implementation Plan: 任务预计耗时

**Branch**: `007-task-estimate` | **Date**: 2026-11-04 | **Spec**: [spec.md](./spec.md)

## Summary

字段与校验都已存在，缺的只是三层接线：存储加一条 `UPDATE`、任务行加一个可点击的耗时标签、新建表单加一个可选输入。

与 005 同构，可以照抄结构：一个 `TaskId` 目标 + 一个草稿字符串 + 一个独立弹窗模块。

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 无新增

**Storage**: 单条 `UPDATE tasks SET estimated_minutes = ?2 WHERE id = ?1 AND deleted_at IS NULL`

**Testing**: 存储层两条集成用例（改值、清空、已删除任务）；界面三条 headless 用例（显示与清除、非法值、改耗时不动其他字段）

**Performance Goals**: 主键单条 `UPDATE`，无新增端点

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | `main_view.rs` 451 行，本次加约 20 行；弹窗独立成 `estimate_dialog.rs`，约 85 行 | PASS |
| II. 质量与可维护性 | 新方法写契约注释与 `# Errors`；不新增 lint 抑制 | PASS |
| III. 测试先行 | 先写存储失败用例，再实现；界面沿用 004 的 headless 方式 | PASS |
| IV. 接触面一致性 | 「预计耗时」文案与错误里的字段名一致；留空与 `0` 的语义复用 `validate_estimated_minutes` | PASS |
| V. 性能与效率 | 单条主键 `UPDATE` | PASS |
| VI. 简单性 | 不做今日清单总耗时汇总；不做单位切换；不参与调度 | PASS |
| VII. 可观测性 | 复用 `OpTrace` 记录 `update_task_estimated_minutes` | PASS |
| VIII. 安全基线 | 参数经 `validate_estimated_minutes`，SQL 绑定 | PASS |

## Project Structure

```text
src/
├── app.rs                  # new_task_estimate、estimate_target、draft_estimate
├── app/estimate_dialog.rs  # 耗时编辑弹窗（新）
├── app/main_view.rs        # 新建表单输入、任务行耗时标签
└── store/task.rs           # update_task_estimated_minutes
src/app/tests/
└── estimate.rs             # 界面验收（新子模块）
specs/007-task-estimate/
├── spec.md、plan.md、tasks.md
```

**Structure Decision**: 弹窗与 `rename_dialog` / `frequency_dialog` / `reset_dialog` 保持同一形态；任务行只在有值时渲染，不给未估时任务增加占位。

## Complexity Tracking

| 选择 | 理由 |
| --- | --- |
| 不把耗时并进「编辑任务」大弹窗 | 那会推翻 004/005 的两个既有弹窗，回归面大于收益 |
| 新建表单的耗时输入不占独立行 | 名称行右侧还有空间，避免表单再长一行 |
| 任务行只在有值时显示 | 693 宽的窗口下每多一个元素都会挤，无值时不占位 |

## Phase 完成情况

- Phase 0: 无新研究项；`estimated_minutes`、`MAX_ESTIMATED_MINUTES` 与 `validate_estimated_minutes` 已存在
- Phase 1: 本文件与 [tasks.md](./tasks.md)
- Phase 2: 待实现
