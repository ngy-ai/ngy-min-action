/// 环境变量前缀：模式读取 `NGY_APP_MODE`，日志目录读取 `NGY_LOG_DIR`，其余同理。
///
/// 前缀隔离可以避免误读其他程序使用的同名变量；`RUST_LOG` 是唯一不带前缀的例外。
pub const ENV_PREFIX: &str = "NGY_";

/// 应用数据目录名，位于用户本地数据目录之下。
pub const APP_DATA_DIR: &str = "ngy-min-action";

/// 任务名称的最大字符数，比目标与分类更宽松，因为任务名往往写得更具体。
pub const MAX_TASK_NAME: usize = 120;

/// `every_n_days` 频率允许的最大间隔天数，覆盖到「一年一次」这种低频习惯。
pub const MAX_EVERY_N_DAYS: u16 = 365;

/// `weekly_n` 频率允许的最大每周次数，等于 7 时退化为每天。
pub const MAX_WEEKLY_N: u8 = 7;

/// 遗忘曲线的复习间隔（天）。第 n 级复习安排在学习日之后第 `REVIEW_INTERVALS_DAYS[n-1]` 天。
///
/// 取经典的艾宾浩斯序列：间隔逐级拉长，既覆盖遗忘最快的阶段，又避免长期高频打扰。
/// 走完全部 6 级视为已巩固，不再自动生成复习行动。
pub const REVIEW_INTERVALS_DAYS: [u16; 6] = [1, 2, 4, 7, 15, 30];

/// 每日行动总数上限的默认值，也是「最小行动」这一概念的默认解释。
pub const DAILY_CAP_DEFAULT: u8 = 5;

/// 每日行动上限允许调整的下界，至少留一条，避免配置成 0 后系统永远不产出行动。
pub const DAILY_CAP_MIN: u8 = 1;

/// 每日行动上限允许调整的上界，超过这个数量就不再是「最小行动」。
pub const DAILY_CAP_MAX: u8 = 20;

/// 每个任务每天最多产出的行动条数。与 `UNIQUE(task_id, on_date)` 的数据库约束一致。
pub const TASKS_PER_DAY_DEFAULT: u8 = 1;

/// 被顺延的行动在原日期保留的天数，超期转 `expired`。
///
/// 不设上限会让连续多日不用后的待办无限堆积，与「不倾倒积压」的目标相悖。
pub const DEFERRED_RETENTION_DAYS: i64 = 7;

/// 跨日检测的轮询间隔（秒）。
///
/// 采用轮询而非精确定时，是为了在系统睡眠唤醒与用户改系统时间两种情况下都不漏触发；
/// 2 秒同时满足「跨日后 5 秒内弹窗」的验收标准。
pub const TICK_INTERVAL_SECS: u64 = 2;

/// 待办备注的最大字符数，按 Unicode 标量计数，换行计入。
pub const NOTE_MAX_CHARS: usize = 2000;

/// 单张图片附件的最大字节数。
pub const IMAGE_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// 图片单边的最大像素数，用于拒绝解压后体积失控的文件。
pub const IMAGE_MAX_DIMENSION: u32 = 12_000;

/// 音频/视频附件的最大字节数。比图片更宽松，因为媒体文件天然更大。
pub const MEDIA_MAX_BYTES: u64 = 200 * 1024 * 1024;

/// 任务内容的最大字符数，比备注更宽松，用于写说明、步骤或带链接的指引。
pub const MAX_TASK_DESCRIPTION: usize = 4000;

/// 历史待办每页的行数。
pub const HISTORY_PAGE_SIZE: u16 = 50;
