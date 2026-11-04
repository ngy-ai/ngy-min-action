# Implementation Plan: 任务频率可修改

**Branch**: `005-editable-task-frequency` | **Date**: 2026-11-04 | **Spec**: [spec.md](./spec.md)

## Summary

任务行加「频率」入口，弹窗预选当前模式与参数；`Store` 增加一个只更新 `freq_kind` 与 `freq_interval` 的方法。生成逻辑完全不动：`is_due` 只比对 `last_generated_on` 与当前频率的间隔天数，频率一改，下一次生成自然按新值判定，既不需要迁移数据，也不需要重算今天。

技术路径：`App` 增加改频率状态（目标任务 + 草稿模式 + 草稿参数），新建 `src/app/frequency_dialog.rs` 承载弹窗，任务行加一个按钮。

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 无新增

**Storage**: SQLite，表结构不变；新增一条 `UPDATE tasks SET freq_kind = ?, freq_interval = ?`

**Testing**: 存储层补集成用例（改成功、非法参数、软删除任务）；界面按 004 的做法用 `src/app/tests.rs` 的 headless 验收

**Performance Goals**: 沿用宪法第 V 条桌面预算（1.18.0）；本 feature 不新增需要测量的端点，单条 `UPDATE` 走主键

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | `main_view.rs` 444 行，本次只加按钮与请求收集，约 +15 行；弹窗独立成 `frequency_dialog.rs`，约 90 行 | PASS |
| II. 质量与可维护性 | 新方法写契约注释与 `# Errors`；不新增 lint 抑制 | PASS |
| III. 测试先行 | 存储方法先写失败用例；界面用 004 已建立的 headless 验收方式 | PASS |
| IV. 接触面一致性 | 弹窗的模式按钮、参数输入与新建任务表单文案一致；错误沿用 `DomainError` 文案 | PASS |
| V. 性能与效率 | 主键单条 `UPDATE`，无循环 IO | PASS |
| VI. 简单性 | 复用既有 `FrequencyMode` 与 `Frequency`，不新造枚举；频率控件在新建表单与弹窗中出现两次，按「重复第二次可以忍」不提取共用组件 | PASS |
| VII. 可观测性 | 复用 `OpTrace` 记录 `update_task_frequency` | PASS |
| VIII. 安全基线 | 参数经 `Frequency::from_parts` 校验，SQL 参数绑定 | PASS |

## Project Structure

```text
src/
├── app.rs                    # frequency_target、draft_frequency_mode、draft_frequency_interval
├── app/frequency_dialog.rs   # 改频率弹窗（新）
├── app/main_view.rs          # 任务行「频率」按钮
└── store/task.rs             # update_task_frequency
tests/
└── store_test.rs             # 改频率、非法参数、软删除任务拒绝
specs/005-editable-task-frequency/
├── spec.md、plan.md、contracts/ui-and-store.md、quickstart.md、tasks.md
```

**Structure Decision**: 弹窗独立成模块，与 `rename_dialog` / `delete_dialog` 对称；`main_view` 继续只负责结构树与任务行。

## Complexity Tracking

| 选择 | 理由 |
| --- | --- |
| 复用 `FrequencyMode` 而非新枚举 | 新建表单已经在用，再定义一个「编辑模式」枚举只会让两个下拉的选项漂移 |
| 不提取共用频率控件 | 这是同一形态第二次出现，按原则 VI 先忍；第三次再提取 |
| 不改 `last_generated_on` | 保持它的原义「最近一次真正生成行动的日期」；改频率不是生成事件 |
| 不提供「立即重算今天」 | 与暂停/恢复一致：参数调整影响未来，不回写已生成的行动 |

## Phase 完成情况

- Phase 0: 无新研究项；`Frequency`、`FrequencyMode` 与 `is_due` 已存在
- Phase 1: [contracts/ui-and-store.md](./contracts/ui-and-store.md)、[quickstart.md](./quickstart.md)
- Phase 2: 待 `/speckit.tasks`
