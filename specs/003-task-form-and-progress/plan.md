# Implementation Plan: 任务频率选择与复习进度展示

**Branch**: `003-task-form-and-progress` | **Date**: 2026-11-03 | **Spec**: [spec.md](./spec.md)

## Summary

把已有的频率模型与复习进度接到界面：新建任务表单提供三种频率模式与参数输入，任务行显示人类可读的频率与复习进度（级别、下次复习日、是否已巩固）。领域层与持久层均已具备能力——`Frequency` 枚举与 `from_parts` 校验、`tasks.freq_kind` / `freq_interval`、`Store::review_progress`——本 feature 只补一处批量读取与界面接线，不改调度语义与表结构。

技术路径：`Frequency::label` 负责频率文案，`Store::progress_for_category` 一次性取出某分类下全部进度，`app` 在刷新任务列表时一并装入进度，任务行按「未开始 / 第 N 级 + 下次复习日 / 已巩固」三种状态渲染。

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 沿用 main 现状，无新增（`eframe` 0.37 / `egui` 0.37）

**Storage**: SQLite 单文件，表结构不变；新增一个只读查询

**Testing**: `cargo test --all-targets --all-features`；频率文案与批量查询在 `tests/store_test.rs` 补用例，界面展示靠手工验收（沿用 001 的 D 决策：不做 GUI 自动化）

**Target Platform**: Windows 10/11 x64

**Performance Goals**: 沿用 001 的桌面预算草案；批量读取替代逐条查询，避免列表渲染出现 N+1

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 已核对：`main_view.rs` 471 行、`app.rs` 444 行、`task.rs` 207 行，预计增加约 80 行，仍低于 600 行预警线 | PASS |
| II. 质量与可维护性 | 新增的公开方法补契约注释；不引入 lint 抑制 | PASS |
| III. 测试先行 | 频率文案与批量查询先写失败用例；界面展示无自动化框架，用手工验收补 | PASS |
| IV. 接触面一致性 | 新增 `Store::progress_for_category` 与 `Frequency::label`，命名与现有 `tasks` / `review_progress` 一致 | PASS |
| V. 性能与效率 | 列表渲染改为一次批量查询，避免每个任务一次 SQL | PASS |
| VI. 简单性与 YAGNI | 不引入新的抽象；展示文案用函数而非 trait；不做「编辑频率」入口 | PASS |
| VII. 可观测性 | 复用 `OpTrace` 记录批量查询耗时，日志走 stderr | PASS |
| VIII. 安全基线 | 参数经 `Frequency::from_parts` 校验，SQL 参数绑定，界面文本走标签控件 | PASS |

## Project Structure

### Documentation (this feature)

```text
specs/003-task-form-and-progress/
├── spec.md
├── plan.md
├── data-model.md
├── contracts/ui-and-store.md
├── quickstart.md
└── tasks.md
```

### Source Code

```text
src/
├── model.rs              # Frequency::label（频率文案）
├── store/task.rs         # progress_for_category（按分类批量读取进度）
└── app/
    ├── app.rs            # 刷新时把进度装入状态
    └── main_view.rs      # 新建任务表单的频率选择 + 任务行的频率与进度展示
tests/
└── store_test.rs         # 频率文案、批量进度读取
```

**Structure Decision**: 展示文案拆成两层——频率是领域概念，放 `model`；进度文案与界面措辞绑定，放 `app`。不改模块边界。

## Complexity Tracking

无破坏性变更、无宪法违规。两点需说明：

| 选择 | 理由 |
| --- | --- |
| 批量读取而不是复用 `review_progress(task)` | 列表一次渲染要 N 条进度，逐条查询是 N+1；宪法 V 明确反对可索引内容的线性查找 |
| 切换频率模式时参数重置为默认值 | 两种模式的合法范围不同（365 vs 7），保留旧值可能直接非法，重置更省事且不会静默出错 |

## Phase 完成情况

- Phase 0: 无新研究项，领域与持久层能力已确认存在（`Frequency`、`from_parts`、`review_progress`）
- Phase 1: [data-model.md](./data-model.md)、[contracts/ui-and-store.md](./contracts/ui-and-store.md)、[quickstart.md](./quickstart.md)
- Phase 2: 待 `/speckit.tasks`
