# Contract: 领域 API

**Feature**: `001-min-action-core` | **Type**: Rust 公共库 API（`ngy_min_action`）

本文件定义对外接触面：类型、函数签名、错误模式。实现必须带契约文档注释（宪法 II / IV）。

## 1. 标识类型

```rust
pub struct GoalId(u64);
pub struct CategoryId(u64);
pub struct TaskId(u64);
pub struct ActionId(u64);
```

**为什么用 newtype**: 四个 ID 底层都是 `u64`，函数签名里混用会静默写到错误的表。这是宪法 VI 允许的「编码了原语不具备的不变量」的包装。

## 2. 领域类型（`model`）

```rust
pub enum Frequency {
    Daily,
    EveryNDays { n: u16 },
    WeeklyN { n: u8 },
}

pub enum TaskStatus { Active, Paused }
pub enum ActionKind { Regular, Review }
pub enum ActionStatus { Pending, Done, Skipped, Deferred, Expired }

pub struct Task {
    pub id: TaskId,
    pub category_id: CategoryId,
    pub name: String,
    pub frequency: Frequency,
    pub estimated_minutes: Option<u16>,
    pub status: TaskStatus,
    pub last_generated_on: Option<NaiveDate>,
}

pub struct ReviewProgress {
    pub task_id: TaskId,
    pub level: u8,
    pub next_review_on: Option<NaiveDate>,
    pub graduated: bool,
}
```

**契约**: 所有 `name` 字段在构造时已通过边界校验（去首尾空白、长度、无控制字符）；类型系统保证进入业务逻辑的名称是干净的。

## 3. 调度（`schedule`）—— 纯函数，无 IO，可基准

```rust
/// 判断任务在 `today` 是否需要生成常规行动。
///
/// 输入：任务频率、`last_generated_on`、目标日期。
/// 输出：`true` 表示需要生成。
/// 错误：无。暂停状态由调用方先过滤，本函数不感知任务状态。
pub fn is_due(task: &Task, today: NaiveDate) -> bool;

/// 计算某一天的完整生成方案。
///
/// 输入：候选任务与复习进度、当天已有行动、每日上限、目标日期。
/// 输出：需要写入的行动（含排序位次）与被顺延的数量。
/// 错误：无；上限为 0 时由调用方保证不传入（配置校验阶段已拒绝）。
/// 保证：同一输入多次调用结果完全一致；同一任务在输出中最多出现一次；
///       复习行动优先于常规行动；输出条数 <= 上限。
pub fn plan_day(input: PlanInput) -> PlanOutput;

/// 完成行动后推进复习进度。
///
/// 输入：当前进度、完成日期、遗忘曲线间隔序列。
/// 输出：新进度；走完序列时 `graduated = true` 且 `next_review_on = None`。
/// 错误：`level` 超出序列长度时返回 `ScheduleError::InvalidLevel`。
pub fn advance_review(
    progress: &ReviewProgress,
    completed_on: NaiveDate,
    intervals: &[u16],
) -> Result<ReviewProgress, ScheduleError>;
```

`PlanInput` 必含：`today`、`daily_cap`、`tasks: &[Task]`、`progress: &[ReviewProgress]`、`existing: &[Action]`。

## 4. 存储（`store`）

```rust
pub struct Store { /* 持有 SQLite 连接 */ }

impl Store {
    /// 打开或创建数据库并完成迁移。
    ///
    /// 错误：`StoreError::Open`（路径不可写）、`StoreError::Migrate`（schema 版本超前）。
    /// 副作用：首次打开时创建数据目录与表结构。
    pub fn open(path: &Path) -> Result<Self, StoreError>;

    pub fn create_goal(&self, name: &str, description: Option<&str>) -> Result<GoalId, DomainError>;
    pub fn create_category(&self, goal: GoalId, name: &str) -> Result<CategoryId, DomainError>;
    pub fn create_task(&self, category: CategoryId, draft: TaskDraft) -> Result<TaskId, DomainError>;

    /// 当日生成，幂等。
    ///
    /// 输入：目标日期与上限。
    /// 输出：写入后的当天行动列表（按 `seq` 排序）。
    /// 保证：重复调用不新增行；已存在的行动保持原状态不变。
    pub fn generate_day(&self, today: NaiveDate, daily_cap: u8) -> Result<Vec<Action>, DomainError>;

    pub fn settle_action(&self, action: ActionId, next: ActionStatus) -> Result<(), DomainError>;
    pub fn undo_last_settlement(&self, action: ActionId) -> Result<(), DomainError>;

    pub fn reminder_state(&self) -> Result<ReminderState, DomainError>;
    pub fn mark_notified(&self, today: NaiveDate) -> Result<(), DomainError>;
    pub fn mark_viewed(&self, today: NaiveDate) -> Result<(), DomainError>;
}
```

**事务边界**: 每个方法内部是一个 SQLite 事务；失败整笔回滚，不留半完成状态（FR-034）。

## 5. 提示（`notify`）

```rust
/// 判断当天是否应弹窗。
///
/// 输入：提示记录、今天、当天待办数量。
/// 输出：应弹窗时为 `true`。
/// 保证：同一 `today` 内连续调用且记录未更新时结果恒定；
///       调用方弹窗后必须立即 `mark_notified`，否则会重复弹窗。
pub fn should_notify(state: &ReminderState, today: NaiveDate, pending: usize) -> bool;
```

**状态转移契约**: `should_notify` 本身无副作用。写入 `last_notified_on` 的职责在调用方，顺序必须是「先判定、再弹窗、弹成功后落库」。

## 6. 错误类型

```rust
pub enum DomainError {
    InvalidName { field: &'static str, reason: NameViolation },
    DuplicateName { scope: &'static str, name: String },
    NotFound { entity: &'static str, id: String },
    Store(StoreError),
}

pub enum NameViolation {
    Empty,
    TooLong { max: usize },
    ControlChar { position: usize },
}
```

**契约（宪法 IV）**: 每个错误必须能回答三件事——哪个输入有问题、期望是什么、如何修正。`Display` 实现按此顺序组织文案，不得只给出「无效输入」这类不可执行的措辞。

## 7. 时钟（`clock`）

```rust
pub trait Clock: Send + Sync {
    fn today(&self) -> NaiveDate;
}

pub struct SystemClock;
```

**契约**: 业务层不得直接调用 `Local::now()`；测试用固定时钟即可覆盖跨日、遗忘曲线与提示去重。
