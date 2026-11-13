//! 领域类型与边界校验。
//!
//! 校验在构造时完成：进入存储层的名称、频率与耗时都已经合法，业务逻辑不再重复判断。

use chrono::NaiveDate;

use crate::common::{MAX_EVERY_N_DAYS, MAX_TASK_NAME, MAX_WEEKLY_N, NOTE_MAX_CHARS};
use crate::store::StoreError;

mod stats;
mod theme;

pub use stats::{ActionTally, ReviewStats};
pub use theme::ThemeChoice;

/// 目标名称的最大字符数。
pub const MAX_GOAL_NAME: usize = 80;

/// 分类名称的最大字符数。
pub const MAX_CATEGORY_NAME: usize = 80;

/// 任务名称的最大字符数。
pub const MAX_TASK_NAME_CHARS: usize = MAX_TASK_NAME;

/// 目标描述的最大字符数。
pub const MAX_DESCRIPTION: usize = 500;

/// 任务内容的最大字符数。比备注更宽松，用于写说明、步骤或带链接的指引。
pub use crate::common::MAX_TASK_DESCRIPTION;

/// 单次预计耗时的上限（分钟）。
pub const MAX_ESTIMATED_MINUTES: u16 = 480;

/// 目标标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GoalId(pub i64);

/// 分类标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategoryId(pub i64);

/// 任务标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub i64);

/// 行动标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub i64);

/// 行动图片标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionImageId(pub i64);

/// 任务内容媒体标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskMediaId(pub i64);

/// 名称不合法的具体原因。每种原因都对应一条可执行的修正建议。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameViolation {
    /// 去空白后为空。
    Empty,
    /// 超长。
    TooLong { max: usize },
    /// 含控制字符，位置从 1 开始计。
    ControlChar { position: usize },
}

impl std::fmt::Display for NameViolation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameViolation::Empty => write!(formatter, "不能为空或只含空格，请填写名称"),
            NameViolation::TooLong { max } => {
                write!(formatter, "长度超过上限 {max} 个字符，请缩短后重试")
            }
            NameViolation::ControlChar { position } => write!(
                formatter,
                "第 {position} 个字符是控制字符，请删除换行、制表符等不可见字符"
            ),
        }
    }
}

/// 领域层错误。`Display` 文案统一按「哪个输入 / 期望什么 / 如何修正」组织。
#[derive(Debug)]
pub enum DomainError {
    /// 名称为空、超长或含控制字符。
    InvalidName {
        field: &'static str,
        violation: NameViolation,
    },
    /// 同一作用域内名称重复。
    DuplicateName { scope: &'static str, name: String },
    /// 数值非法。
    InvalidValue {
        field: &'static str,
        expected: String,
    },
    /// 目标对象不存在。
    NotFound { entity: &'static str, id: i64 },
    /// 存储层错误。
    Store(StoreError),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::InvalidName { field, violation } => {
                write!(formatter, "{field} 不合法：{violation}")
            }
            DomainError::DuplicateName { scope, name } => write!(
                formatter,
                "{scope}：已存在名称「{name}」，请换一个名称或先删除原有的那一项"
            ),
            DomainError::InvalidValue { field, expected } => {
                write!(formatter, "{field} 不合法：期望 {expected}")
            }
            DomainError::NotFound { entity, id } => {
                write!(
                    formatter,
                    "{entity} {id} 不存在，可能已被删除，请刷新后重试"
                )
            }
            DomainError::Store(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for DomainError {}

impl From<StoreError> for DomainError {
    fn from(error: StoreError) -> Self {
        DomainError::Store(error)
    }
}

/// 目标：一个长期方向。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    /// 标识。
    pub id: GoalId,
    /// 名称，已通过边界校验。
    pub name: String,
    /// 可选描述。
    pub description: Option<String>,
    /// 创建时间，RFC 3339。
    pub created_at: String,
}

/// 分类：目标下的一类能力或主题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// 标识。
    pub id: CategoryId,
    /// 所属目标。
    pub goal_id: GoalId,
    /// 名称，同一目标内唯一。
    pub name: String,
    /// 创建时间，RFC 3339。
    pub created_at: String,
}

/// 任务频率。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    /// 每天一次。
    Daily,
    /// 每 N 天一次。
    EveryNDays(u16),
    /// 每周 N 次。
    WeeklyN(u8),
}

impl Frequency {
    /// 两次生成之间的最小间隔天数。
    ///
    /// `WeeklyN` 取 `⌊7 / n⌋`，因此 `n = 7` 时退化为每天。
    #[must_use]
    pub fn interval_days(self) -> i64 {
        match self {
            Frequency::Daily => 1,
            Frequency::EveryNDays(days) => i64::from(days.max(1)),
            Frequency::WeeklyN(times) => {
                let times = i64::from(times.clamp(1, MAX_WEEKLY_N));
                (7 / times).max(1)
            }
        }
    }

    /// 人类可读的频率描述。
    ///
    /// 只用于展示，不参与调度判定。
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Frequency::Daily => "每天".to_string(),
            Frequency::EveryNDays(days) => format!("每 {days} 天"),
            Frequency::WeeklyN(times) => format!("每周 {times} 次"),
        }
    }

    /// 入库用的类型标签。
    #[must_use]
    pub fn kind(self) -> &'static str {
        match self {
            Frequency::Daily => "daily",
            Frequency::EveryNDays(_) => "every_n_days",
            Frequency::WeeklyN(_) => "weekly_n",
        }
    }

    /// 入库用的原始间隔值。
    #[must_use]
    pub fn raw_interval(self) -> i64 {
        match self {
            Frequency::Daily => 1,
            Frequency::EveryNDays(days) => i64::from(days),
            Frequency::WeeklyN(times) => i64::from(times),
        }
    }

    /// 从入库值还原频率，并做边界校验。
    ///
    /// # Errors
    ///
    /// 类型标签未知或间隔超出允许范围时返回 [`DomainError::InvalidValue`]。
    pub fn from_parts(kind: &str, interval: i64) -> Result<Self, DomainError> {
        match kind {
            "daily" => Ok(Frequency::Daily),
            "every_n_days" => {
                let days = u16::try_from(interval).map_err(|_| DomainError::InvalidValue {
                    field: "频率间隔",
                    expected: format!("1 到 {MAX_EVERY_N_DAYS} 之间的整数"),
                })?;
                if days == 0 || days > MAX_EVERY_N_DAYS {
                    return Err(DomainError::InvalidValue {
                        field: "频率间隔",
                        expected: format!("1 到 {MAX_EVERY_N_DAYS} 之间的整数"),
                    });
                }
                Ok(Frequency::EveryNDays(days))
            }
            "weekly_n" => {
                let times = u8::try_from(interval).map_err(|_| DomainError::InvalidValue {
                    field: "每周次数",
                    expected: format!("1 到 {MAX_WEEKLY_N} 之间的整数"),
                })?;
                if times == 0 || times > MAX_WEEKLY_N {
                    return Err(DomainError::InvalidValue {
                        field: "每周次数",
                        expected: format!("1 到 {MAX_WEEKLY_N} 之间的整数"),
                    });
                }
                Ok(Frequency::WeeklyN(times))
            }
            other => Err(DomainError::InvalidValue {
                field: "频率类型",
                expected: format!("daily、every_n_days 或 weekly_n，实际为 {other}"),
            }),
        }
    }
}

/// 任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 进行中，参与每日生成。
    Active,
    /// 已暂停，不参与每日生成。
    Paused,
}

impl TaskStatus {
    /// 入库用的标签。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Active => "active",
            TaskStatus::Paused => "paused",
        }
    }

    /// 从入库值还原。
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "paused" => TaskStatus::Paused,
            _ => TaskStatus::Active,
        }
    }
}

/// 任务：一个可重复执行的最小单元。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// 标识。
    pub id: TaskId,
    /// 所属分类。
    pub category_id: CategoryId,
    /// 名称。
    pub name: String,
    /// 频率。
    pub frequency: Frequency,
    /// 可选的单次预计耗时（分钟）。
    pub estimated_minutes: Option<u16>,
    /// 可选的内容：富纯文本与媒体附件的说明，去掉首尾空白后为空即 `None`。
    pub description: Option<String>,
    /// 状态。
    pub status: TaskStatus,
    /// 最近一次生成常规行动的日期。
    pub last_generated_on: Option<NaiveDate>,
    /// 创建时间，RFC 3339。用于同优先级下的稳定排序。
    pub created_at: String,
}

/// 新建任务时的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDraft {
    /// 名称。
    pub name: String,
    /// 频率。
    pub frequency: Frequency,
    /// 可选的单次预计耗时（分钟）。
    pub estimated_minutes: Option<u16>,
    /// 可选的内容：富纯文本与媒体附件的说明，去掉首尾空白后为空即 `None`。
    pub description: Option<String>,
}

/// 行动类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// 由频率产生的常规行动。
    Regular,
    /// 由遗忘曲线产生的复习行动。
    Review,
}

impl ActionKind {
    /// 入库用的标签。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::Regular => "regular",
            ActionKind::Review => "review",
        }
    }

    /// 从入库值还原。
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "review" => ActionKind::Review,
            _ => ActionKind::Regular,
        }
    }
}

/// 行动状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    /// 待完成。
    Pending,
    /// 已完成。
    Done,
    /// 已跳过。
    Skipped,
    /// 因当日上限被顺延，仅作记录。
    Deferred,
    /// 顺延超期，不再参与竞争。
    Expired,
}

impl ActionStatus {
    /// 该状态是否由一次结算产生，因而存在可撤销的快照。
    ///
    /// 只有完成与跳过会写入 `action_settlements`；顺延与过期从未结算，
    /// 界面上不能给它们撤销入口。
    #[must_use]
    pub fn is_settled(self) -> bool {
        matches!(self, ActionStatus::Done | ActionStatus::Skipped)
    }

    /// 入库用的标签。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ActionStatus::Pending => "pending",
            ActionStatus::Done => "done",
            ActionStatus::Skipped => "skipped",
            ActionStatus::Deferred => "deferred",
            ActionStatus::Expired => "expired",
        }
    }

    /// 从入库值还原。
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "done" => ActionStatus::Done,
            "skipped" => ActionStatus::Skipped,
            "deferred" => ActionStatus::Deferred,
            "expired" => ActionStatus::Expired,
            _ => ActionStatus::Pending,
        }
    }
}

/// 行动：某一天的一条具体待办。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// 标识。
    pub id: ActionId,
    /// 所属任务。
    pub task_id: TaskId,
    /// 所属日期。
    pub on_date: NaiveDate,
    /// 类型。
    pub kind: ActionKind,
    /// 复习级别，仅复习行动有值。
    pub review_level: Option<u8>,
    /// 状态。
    pub status: ActionStatus,
    /// 当日排序位次。
    pub seq: u32,
    /// 用户的文字备注，去掉首尾空白后为空即 `None`。
    pub note: Option<String>,
}

/// 行动的一张图片附件。磁盘副本位于数据目录的 `attachments/<行动 ID>/`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionImage {
    /// 标识。
    pub id: ActionImageId,
    /// 所属行动。
    pub action_id: ActionId,
    /// 数据目录内的文件名，由程序生成，不含路径分隔符。
    pub file_name: String,
    /// 用户原始文件名，仅用于展示。
    pub original_name: String,
    /// 文件字节数。
    pub byte_size: u64,
    /// 加入时间，RFC 3339。
    pub created_at: String,
}

/// 任务内容的媒体类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskMediaKind {
    /// 图片，展示时内联渲染。
    Image,
    /// 音频，点击后由系统默认程序打开。
    Audio,
    /// 视频，点击后由系统默认程序打开。
    Video,
}

impl TaskMediaKind {
    /// 入库用的标签。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TaskMediaKind::Image => "image",
            TaskMediaKind::Audio => "audio",
            TaskMediaKind::Video => "video",
        }
    }

    /// 从入库值还原。未知标签归为图片，避免既有数据被丢弃。
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "audio" => TaskMediaKind::Audio,
            "video" => TaskMediaKind::Video,
            _ => TaskMediaKind::Image,
        }
    }
}

/// 任务内容的一段媒体附件。磁盘副本位于数据目录的 `task_content/<任务 ID>/`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMedia {
    /// 标识。
    pub id: TaskMediaId,
    /// 所属任务。
    pub task_id: TaskId,
    /// 媒体类型。
    pub kind: TaskMediaKind,
    /// 数据目录内的文件名，由程序生成，不含路径分隔符。
    pub file_name: String,
    /// 用户原始文件名，仅用于展示。
    pub original_name: String,
    /// 文件字节数。
    pub byte_size: u64,
    /// 加入时间，RFC 3339。
    pub created_at: String,
}

/// 历史待办的一页。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPage {
    /// 本页行动，按日期倒序、同日按顺序倒序。
    pub actions: Vec<Action>,
    /// 继续向前翻页的游标：日期与当日顺序位次。
    pub next_cursor: Option<(NaiveDate, i64)>,
    /// 是否还有更早的行。
    pub has_more: bool,
}

pub use crate::review::*;

/// 校验并规范化名称。
///
/// # Errors
///
/// 去首尾空白后为空、含控制字符或超过 `max` 个字符时返回
/// [`DomainError::InvalidName`]，`field` 用于指出是哪一项输入。
pub fn validate_name(raw: &str, field: &'static str, max: usize) -> Result<String, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::InvalidName {
            field,
            violation: NameViolation::Empty,
        });
    }
    let mut count = 0_usize;
    for (index, character) in trimmed.chars().enumerate() {
        if character.is_control() {
            return Err(DomainError::InvalidName {
                field,
                violation: NameViolation::ControlChar {
                    position: index + 1,
                },
            });
        }
        count += 1;
    }
    if count > max {
        return Err(DomainError::InvalidName {
            field,
            violation: NameViolation::TooLong { max },
        });
    }
    Ok(trimmed.to_owned())
}

/// 计算名称的比较键：Unicode 默认大小写折叠。
///
/// 输入须已通过 [`validate_name`]，即两端空白已去除。纯函数：同一输入恒返回
/// 同一输出，且对折叠结果再次折叠不变。
#[must_use]
pub fn fold_name(name: &str) -> String {
    caseless::default_case_fold_str(name)
}

/// 校验预计耗时。
///
/// # Errors
///
/// 超出 `1..=MAX_ESTIMATED_MINUTES` 时返回 [`DomainError::InvalidValue`]。
pub fn validate_estimated_minutes(minutes: Option<u16>) -> Result<Option<u16>, DomainError> {
    match minutes {
        None | Some(0) => Ok(None),
        Some(value) if value <= MAX_ESTIMATED_MINUTES => Ok(Some(value)),
        Some(value) => Err(DomainError::InvalidValue {
            field: "预计耗时",
            expected: format!("1 到 {MAX_ESTIMATED_MINUTES} 分钟，或留空；实际为 {value} 分钟"),
        }),
    }
}

/// 校验并规范化待办备注。
///
/// 返回值是入库形态：去掉首尾空白后为空即 `None`，表示清除备注；否则保留内部换行与
/// 制表符。允许换行（备注可以分点），拒绝其它控制字符（避免日志与界面被注入）。
///
/// # Errors
///
/// 超过 [`NOTE_MAX_CHARS`] 个字符或含不允许的控制字符时返回
/// [`DomainError::InvalidValue`]。
pub fn validate_note(raw: &str) -> Result<Option<String>, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > NOTE_MAX_CHARS {
        return Err(DomainError::InvalidValue {
            field: "备注",
            expected: format!("不超过 {NOTE_MAX_CHARS} 个字符"),
        });
    }
    if trimmed
        .chars()
        .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(DomainError::InvalidValue {
            field: "备注",
            expected: "不含换行与制表符之外的不可见控制字符".to_owned(),
        });
    }
    Ok(Some(trimmed.to_owned()))
}

/// 校验并规范化任务内容。
///
/// 与 [`validate_note`] 同源：去掉首尾空白后为空即 `None`，表示清除内容；允许内部换行
/// 与制表符，拒绝其它控制字符（避免日志与界面被注入）。上限为 [`MAX_TASK_DESCRIPTION`]。
///
/// # Errors
///
/// 超过 [`MAX_TASK_DESCRIPTION`] 个字符或含不允许的控制字符时返回
/// [`DomainError::InvalidValue`]。
pub fn validate_description(raw: &str) -> Result<Option<String>, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_TASK_DESCRIPTION {
        return Err(DomainError::InvalidValue {
            field: "任务内容",
            expected: format!("不超过 {MAX_TASK_DESCRIPTION} 个字符"),
        });
    }
    if trimmed
        .chars()
        .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(DomainError::InvalidValue {
            field: "任务内容",
            expected: "不含换行与制表符之外的不可见控制字符".to_owned(),
        });
    }
    Ok(Some(trimmed.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Frequency, fold_name};

    #[test]
    fn frequency_labels_are_human_readable() {
        assert_eq!(Frequency::Daily.label(), "每天");
        assert_eq!(Frequency::EveryNDays(3).label(), "每 3 天");
        assert_eq!(Frequency::WeeklyN(2).label(), "每周 2 次");
    }

    #[test]
    fn frequency_parameters_are_bounded() {
        assert!(Frequency::from_parts("every_n_days", 0).is_err());
        assert!(Frequency::from_parts("every_n_days", 366).is_err());
        assert!(Frequency::from_parts("weekly_n", 0).is_err());
        assert!(Frequency::from_parts("weekly_n", 8).is_err());
        assert_eq!(
            Frequency::from_parts("every_n_days", 3).expect("合法间隔"),
            Frequency::EveryNDays(3)
        );
        assert_eq!(
            Frequency::from_parts("weekly_n", 7).expect("合法次数"),
            Frequency::WeeklyN(7)
        );
    }

    #[test]
    fn sharp_s_expands_under_full_case_folding() {
        assert_eq!(fold_name("straße"), "strasse", "ß 必须按完整折叠展开为 ss");
    }

    #[test]
    fn greek_upper_and_lower_fold_to_the_same_key() {
        assert_eq!(fold_name("ΣΟΦΟΣ"), fold_name("σοφος"));
    }

    #[test]
    fn chinese_names_survive_folding_unchanged() {
        assert_eq!(fold_name("口语"), "口语");
    }

    #[test]
    fn folding_is_idempotent() {
        let once = fold_name("Straße");
        assert_eq!(fold_name(&once), once);
    }
}
