//! 启动位置：默认数据目录与自定义位置的持久化。
//!
//! 位置文件固定放在启动目录（默认是用户本地数据目录下的 `ngy-min-action`），
//! 与数据目录解耦；这样设置新位置后，程序仍知道去哪里读配置。

use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::common::APP_DATA_DIR;

/// 记录自定义数据目录的文件名，内容是一行绝对路径。
const LOCATION_FILE: &str = "location";
/// 单实例锁文件名，固定在启动目录，与数据位置无关。
const LOCK_FILE: &str = "app.lock";
/// 写入位置文件时使用的临时文件名。
const TEMP_FILE: &str = "location.tmp";

/// 启动目录与数据目录的解析器。
#[derive(Debug, Clone)]
pub struct Bootstrap {
    dir: PathBuf,
}

impl Bootstrap {
    /// 使用系统默认启动目录：`%LOCALAPPDATA%\ngy-min-action`，读不到环境变量时退回临时目录。
    #[must_use]
    pub fn system() -> Self {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(APP_DATA_DIR)
            .into()
    }

    /// 使用指定目录作为启动目录，主要供测试注入。
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// 启动目录。
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 单实例锁路径。
    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.dir.join(LOCK_FILE)
    }

    /// 位置文件路径。
    #[must_use]
    pub fn location_path(&self) -> PathBuf {
        self.dir.join(LOCATION_FILE)
    }

    /// 未配置自定义位置时的数据目录，也是默认位置。
    #[must_use]
    pub fn default_data_dir(&self) -> PathBuf {
        self.dir.clone()
    }

    /// 当前数据目录。
    ///
    /// 位置文件缺失、为空、含相对路径或不可读时回退默认目录并记录一条 warn；
    /// 启动不因配置损坏而失败。
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        match self.read_location() {
            Ok(Some(path)) => path,
            Ok(None) => self.default_data_dir(),
            Err(error) => {
                warn!(
                    op = "config_read",
                    result = "fallback",
                    error = %error,
                    "位置配置不可用，回退默认目录"
                );
                self.default_data_dir()
            }
        }
    }

    /// 把数据目录写入位置文件。
    ///
    /// 先创建目标目录并验证可写，再写临时文件并替换；失败时旧配置保持不变。
    ///
    /// # Errors
    ///
    /// 路径不是绝对路径、目标不可创建或不可写、位置文件写入失败时返回
    /// [`ConfigError`]，文案说明如何修正。
    pub fn set_data_dir(&self, path: &Path) -> Result<(), ConfigError> {
        if !path.is_absolute() {
            return Err(ConfigError::RelativePath {
                path: path.display().to_string(),
            });
        }
        std::fs::create_dir_all(path).map_err(|error| ConfigError::Write {
            path: path.display().to_string(),
            reason: format!("无法创建目录：{error}"),
        })?;
        if !path.is_dir() {
            return Err(ConfigError::Write {
                path: path.display().to_string(),
                reason: "目标不是目录".to_owned(),
            });
        }
        std::fs::create_dir_all(&self.dir).map_err(|error| ConfigError::Write {
            path: self.dir.display().to_string(),
            reason: format!("无法创建启动目录：{error}"),
        })?;

        let temporary = self.dir.join(TEMP_FILE);
        std::fs::write(&temporary, path.to_string_lossy().as_bytes()).map_err(|error| {
            ConfigError::Write {
                path: temporary.display().to_string(),
                reason: format!("无法写入临时位置文件：{error}"),
            }
        })?;
        let location = self.location_path();
        if location.exists() {
            std::fs::remove_file(&location).map_err(|error| ConfigError::Write {
                path: location.display().to_string(),
                reason: format!("无法替换旧位置文件：{error}"),
            })?;
        }
        std::fs::rename(&temporary, &location).map_err(|error| ConfigError::Write {
            path: location.display().to_string(),
            reason: format!("无法保存位置文件：{error}"),
        })?;
        info!(
            op = "config_write",
            result = "ok",
            data_dir = %path.display(),
            "数据位置已更新"
        );
        Ok(())
    }

    fn read_location(&self) -> Result<Option<PathBuf>, ConfigError> {
        let path = self.location_path();
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(ConfigError::Read {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                });
            }
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let candidate = PathBuf::from(trimmed);
        if !candidate.is_absolute() {
            return Err(ConfigError::RelativePath {
                path: trimmed.to_owned(),
            });
        }
        Ok(Some(candidate))
    }
}

impl From<PathBuf> for Bootstrap {
    fn from(dir: PathBuf) -> Self {
        Self::new(dir)
    }
}

/// 位置配置错误。文案统一说明问题与修正方式。
#[derive(Debug)]
pub enum ConfigError {
    /// 位置文件里不是绝对路径。
    RelativePath {
        /// 实际读到的内容。
        path: String,
    },
    /// 读取位置文件失败。
    Read {
        /// 文件路径。
        path: String,
        /// 失败原因。
        reason: String,
    },
    /// 创建或写入位置文件失败。
    Write {
        /// 路径。
        path: String,
        /// 失败原因。
        reason: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::RelativePath { path } => {
                write!(formatter, "数据位置「{path}」不是绝对路径，请重新选择目录")
            }
            ConfigError::Read { path, reason } => write!(
                formatter,
                "无法读取位置配置 {path}：{reason}。程序已回退默认目录，请检查文件权限"
            ),
            ConfigError::Write { path, reason } => {
                write!(formatter, "无法保存数据位置 {path}：{reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}
