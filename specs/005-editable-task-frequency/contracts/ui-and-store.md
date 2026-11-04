# Phase 1 Contract: 改频率弹窗与存储方法

**Feature**: `005-editable-task-frequency` | **Date**: 2026-11-04

## Store

```rust
/// 更新任务的频率。
///
/// 只改 `freq_kind` 与 `freq_interval`；名称、状态、`last_generated_on` 与
/// 复习进度都不变。已生成的行动不追溯，下一次 `generate_day` 按新间隔判定。
///
/// # Errors
///
/// 任务不存在或已删除时返回 [`DomainError::NotFound`]。
pub fn update_task_frequency(&self, task: TaskId, frequency: Frequency) -> Result<(), DomainError>;
```

**契约**:

- 参数合法性由 `Frequency::from_parts` 在调用侧保证，存储层不再重复校验，与 `create_task` 一致
- 软删除的任务返回 `NotFound`，不改任何行
- 不触碰 `actions`、`review_progress` 与 `last_generated_on`
- 单条 `UPDATE`，条件 `deleted_at IS NULL`

## 界面交互

### 改频率

| 项 | 说明 |
| --- | --- |
| 入口 | 任务行「频率」按钮 |
| 弹窗标题 | 「修改频率」 |
| 预选 | 打开时模式与参数预填为该任务的当前值 |
| 模式 | 每天 / 每 N 天 / 每周 N 次，与新建任务表单同一组文案 |
| 参数输入 | 仅「每 N 天」「每周 N 次」显示，标签与新建表单一致 |
| 按钮 | 「保存」「取消」 |
| 失败 | 弹窗保留、输入保留，错误条在顶部显示 |

**边界**:

- 弹窗打开期间切换左侧选中的分类，弹窗继续指向原任务
- 暂停中的任务允许保存，保存后仍为暂停
- 改成与当前值相同的频率视为正常保存，不报错

## 错误文案

沿用既有 `DomainError` 渲染：

- 参数越界：`频率间隔 不合法：期望 1 到 365 之间的整数`
- 每周次数越界：`每周次数 不合法：期望 1 到 7 之间的整数`
- 任务已被删除：`任务 12 不存在，可能已被删除，请刷新后重试`

## 不改动的部分

- `src/schedule/` 全部逻辑：`is_due` 已经按当前频率计算
- 表结构与迁移版本
- 新建任务表单
