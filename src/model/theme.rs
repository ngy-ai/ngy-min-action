//! 主题偏好：跟随系统、浅色、深色。

/// 用户选择的界面主题。
///
/// 该类型属于领域层，不依赖界面框架；界面层负责把它映射成具体的主题设置。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeChoice {
    /// 跟随操作系统明暗，也是首次启动的默认值。
    #[default]
    System,
    /// 固定浅色。
    Light,
    /// 固定深色。
    Dark,
}

impl ThemeChoice {
    /// 持久化取值：`system` / `light` / `dark`。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// 解析持久化取值；空串、空白与任何无法识别的值都回退 [`Self::System`]。
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}
