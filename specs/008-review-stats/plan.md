# Implementation Plan: 复习统计与回顾

**Branch**: `008-review-stats` | **Date**: 2026-11-04 | **Spec**: [spec.md](./spec.md)

## Summary

数据都在库里，只缺一次聚合。新增 `src/store/stats.rs` 提供 `review_stats(today, window_days)`，内部三条只读查询：窗口内分项计数、按天聚合算连续天数、复习阶段分布。界面加一个「统计」按钮与一个只读面板，打开时查询一次并缓存结果，不做每帧查询。

行动统计不过滤软删除任务：过期行动只可能由「删除任务」产生，若把这批行动一并排除，过期数恒为 0。阶段分布则只看当前任务库，软删除的任务不参与。

## Technical Context

**Language/Version**: Rust 1.97，edition 2024

**Primary Dependencies**: 无新增

**Storage**: 三条 `SELECT`，不改表、不写入

**Testing**: 存储层覆盖分项、连续天数四种情形、阶段分布、软删除排除；界面覆盖快照填充与空状态

**Performance Goals**: 打开面板时三条聚合查询，每帧零查询；30 天窗口下数据量为「行动数 × 天数」级别，桌面端可忽略

## 数据结构

```rust
/// 窗口内已结算行动的分项计数。
pub struct ActionTally {
    pub done: usize,
    pub skipped: usize,
    pub expired: usize,
}

impl ActionTally {
    /// 已结算合计；为零时说明窗口内没有可统计的行动。
    pub fn settled(&self) -> usize;
    /// 完成率百分比；已结算为零时返回 `None`。
    pub fn completion_percent(&self) -> Option<u32>;
}

/// 一次统计的全部结果。
pub struct ReviewStats {
    pub window_days: u32,
    pub tally: ActionTally,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub untouched_tasks: usize,
    pub learning_tasks: usize,
    pub graduated_tasks: usize,
}
```

## 连续天数算法

按 `on_date` 倒序遍历「有行动的日子」：

- `done > 0` → 当前连续 +1
- `done == 0` 且该日早于今天 → 中断，停止累加
- `done == 0` 且该日就是今天 → 跳过，继续看前一天
- 没有行动的日子不出现于聚合结果中，自然不影响

最长连续在同一次遍历里对「连续达成段」取最大值。

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 新增 `store/stats.rs` 约 190 行、`app/stats_dialog.rs` 约 160 行；`main_view.rs` 只加一个按钮（+6 行） | PASS |
| II. 质量与可维护性 | 三条查询各自独立函数、带 `# Errors`；`ActionTally` 承担除零逻辑，界面不做除法 | PASS |
| III. 测试先行 | 先写 `review_stats` 的失败用例（含四种连续天数情形），再实现 | PASS |
| IV. 接触面一致性 | 字段名与错误文案沿用既有中文术语；空状态文案由 spec 固定 | PASS |
| V. 性能与效率 | 打开时三条查询 + 一次缓存；不为每帧重查 | PASS |
| VI. 简单性 | 固定 30 天窗口、不做图表、不做按目标分组、不做缓存失效 | PASS |
| VII. 可观测性 | 聚合查询复用 `OpTrace` | PASS |
| VIII. 安全基线 | 只读查询，参数绑定，无写入 | PASS |

## Project Structure

```text
src/
├── store.rs               # mod stats; 转出 ReviewStats
├── store/stats.rs         # review_stats 与三条聚合（新）
├── app.rs                 # show_stats、stats 快照状态
├── app/stats_dialog.rs    # 统计面板（新）
├── app/main_view.rs       # 状态条加「统计」按钮
└── app/tests/stats.rs     # 界面验收（新子模块）
tests/store_test.rs        # 统计的存储层用例
specs/008-review-stats/
├── spec.md、plan.md、tasks.md
```

**Structure Decision**: 存储层单开 `stats.rs`，不塞进 `action.rs`；界面单开 `stats_dialog.rs`，与其余四个弹窗同构。

## Complexity Tracking

| 选择 | 理由 |
| --- | --- |
| 打开时查询并缓存 | 每帧三条聚合查询虽然不慢，但纯属浪费；缓存失效只在重新打开时发生，语义也更好 |
| 不引入比率类型 | `completion_percent() -> Option<u32>` 就够，`None` 天然表示空状态 |
| 连续天数在 Rust 里算 | SQL 窗口函数算 streak 可读性差且 SQLite 版本依赖强，按天聚合后线性遍历更清楚 |
| 不统计单目标 | 会把查询翻倍且需要另一套界面，留给下一个 feature |

## Phase 完成情况

- Phase 0: 无新研究项；`actions`、`review_progress`、软删除标记都已就位
- Phase 1: 本文件与 [tasks.md](./tasks.md)
- Phase 2: 待实现
