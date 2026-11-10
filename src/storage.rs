//! 数据位置迁移：复制数据库与附件、校验、写位置配置，再由调用方替换连接并清理旧位置。
//!
//! 迁移的顺序不可交换：只有新库能打开且完整性检查通过后，才把新位置写入配置；
//! 任何一步失败都回收目标目录里的临时产物，旧位置保持可用。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{info, warn};

use crate::common::elapsed_ms;
use crate::config::{Bootstrap, ConfigError};
use crate::store::Store;

/// 迁移成功后的新连接与新位置。
#[derive(Debug)]
pub struct Relocation {
    /// 指向新数据库的连接。
    pub store: Store,
    /// 新的数据目录。
    pub data_dir: PathBuf,
}

/// 把 `store` 的数据迁移到 `new_dir`。
///
/// 调用方在成功后用 [`Relocation::store`] 替换旧连接，再调用
/// [`cleanup_old_data`] 清理旧位置；迁移失败时旧连接与旧数据都不受影响。
///
/// # Errors
///
/// 目标不是绝对路径、与源相同或互相嵌套、目标已有数据、目录不可写、备份或复制
/// 失败、完整性检查失败、位置配置写入失败时返回 [`StorageError`]。
pub fn relocate(
    store: &Store,
    new_dir: &Path,
    bootstrap: &Bootstrap,
) -> Result<Relocation, StorageError> {
    let started = Instant::now();
    let old_dir = store.data_dir().to_path_buf();
    validate_target(&old_dir, new_dir)?;

    fs::create_dir_all(new_dir).map_err(|error| StorageError::Target {
        path: new_dir.display().to_string(),
        reason: format!("无法创建目录：{error}"),
    })?;
    probe_writable(new_dir)?;

    let new_database = new_dir.join("data.db");
    let new_attachments = new_dir.join("attachments");
    let mut copied_attachments = false;

    let result = (|| -> Result<Store, StorageError> {
        store
            .backup_to(&new_database)
            .map_err(|error| StorageError::Backup {
                path: new_database.display().to_string(),
                reason: error.to_string(),
            })?;

        if old_dir.join("attachments").is_dir() {
            copy_tree(&old_dir.join("attachments"), &new_attachments)?;
            copied_attachments = true;
        }

        let verified = Store::open(&new_database).map_err(|error| StorageError::Verify {
            path: new_database.display().to_string(),
            reason: error.to_string(),
        })?;
        let intact = verified
            .integrity_ok()
            .map_err(|error| StorageError::Verify {
                path: new_database.display().to_string(),
                reason: error.to_string(),
            })?;
        if !intact {
            return Err(StorageError::Verify {
                path: new_database.display().to_string(),
                reason: "SQLite 完整性检查未通过".to_owned(),
            });
        }

        bootstrap
            .set_data_dir(new_dir)
            .map_err(StorageError::Config)?;
        Ok(verified)
    })();

    match result {
        Ok(verified) => {
            info!(
                op = "storage_relocate",
                result = "ok",
                from = %old_dir.display(),
                to = %new_dir.display(),
                elapsed_ms = elapsed_ms(started),
                "数据位置已迁移"
            );
            Ok(Relocation {
                store: verified,
                data_dir: new_dir.to_path_buf(),
            })
        }
        Err(error) => {
            rollback_target(
                &new_database,
                copied_attachments.then_some(new_attachments.as_path()),
            );
            warn!(
                op = "storage_relocate",
                result = "error",
                from = %old_dir.display(),
                to = %new_dir.display(),
                elapsed_ms = elapsed_ms(started),
                error = %error,
                "数据迁移失败，保留原位置"
            );
            Err(error)
        }
    }
}

/// 删除旧位置的数据文件：数据库、WAL/SHM 与附件目录。
///
/// 只删除已知的数据产物，不碰位置配置与单实例锁；默认位置与启动目录重合时，
/// 因此不会把配置删掉。
///
/// # Errors
///
/// 已知数据文件存在但删除失败时返回 [`StorageError::Cleanup`]。
pub fn cleanup_old_data(dir: &Path) -> Result<(), StorageError> {
    for name in ["data.db", "data.db-wal", "data.db-shm"] {
        let path = dir.join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(StorageError::Cleanup {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                });
            }
        }
    }
    let attachments = dir.join("attachments");
    match fs::remove_dir_all(&attachments) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(StorageError::Cleanup {
                path: attachments.display().to_string(),
                reason: error.to_string(),
            });
        }
    }
    info!(
        op = "storage_cleanup",
        result = "ok",
        dir = %dir.display(),
        "旧位置数据已清理"
    );
    Ok(())
}

fn validate_target(old_dir: &Path, new_dir: &Path) -> Result<(), StorageError> {
    if !new_dir.is_absolute() {
        return Err(StorageError::Target {
            path: new_dir.display().to_string(),
            reason: "目标必须是绝对路径".to_owned(),
        });
    }
    // 用 `absolute` 而不是 `canonicalize`：目标目录可能还不存在，且 Windows 的
    // canonicalize 会带 `\\?\` 前缀，直接与用户路径比较会漏掉嵌套判断。
    let old_absolute = std::path::absolute(old_dir).unwrap_or_else(|_| old_dir.to_path_buf());
    let new_absolute = std::path::absolute(new_dir).unwrap_or_else(|_| new_dir.to_path_buf());
    if new_absolute == old_absolute {
        return Err(StorageError::Target {
            path: new_dir.display().to_string(),
            reason: "目标与当前数据位置相同，请选择另一个目录".to_owned(),
        });
    }
    if new_absolute.starts_with(&old_absolute) || old_absolute.starts_with(&new_absolute) {
        return Err(StorageError::Target {
            path: new_dir.display().to_string(),
            reason: "目标不能位于当前数据目录内，也不能包含当前数据目录".to_owned(),
        });
    }
    if new_dir.join("data.db").exists() {
        return Err(StorageError::Target {
            path: new_dir.display().to_string(),
            reason: "目标目录已有 data.db，请选择空目录，避免覆盖已有数据".to_owned(),
        });
    }
    let attachments = new_dir.join("attachments");
    if attachments.exists() && directory_has_entries(&attachments) {
        return Err(StorageError::Target {
            path: new_dir.display().to_string(),
            reason: "目标目录已有非空 attachments，请选择空目录，避免混合两批附件".to_owned(),
        });
    }
    Ok(())
}

fn probe_writable(dir: &Path) -> Result<(), StorageError> {
    let probe = dir.join(format!(".ngy-migrate-probe-{}", std::process::id()));
    let result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe);
    match result {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(error) => Err(StorageError::Target {
            path: dir.display().to_string(),
            reason: format!("目录不可写：{error}"),
        }),
    }
}

fn directory_has_entries(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(true)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(to).map_err(|error| StorageError::Copy {
        path: to.display().to_string(),
        reason: format!("无法创建附件目录：{error}"),
    })?;
    let entries = fs::read_dir(from).map_err(|error| StorageError::Copy {
        path: from.display().to_string(),
        reason: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| StorageError::Copy {
            path: from.display().to_string(),
            reason: error.to_string(),
        })?;
        let file_type = entry.file_type().map_err(|error| StorageError::Copy {
            path: entry.path().display().to_string(),
            reason: error.to_string(),
        })?;
        if file_type.is_symlink() {
            warn!(
                op = "storage_copy",
                result = "skipped",
                path = %entry.path().display(),
                "跳过符号链接"
            );
            continue;
        }
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| StorageError::Copy {
                path: target.display().to_string(),
                reason: error.to_string(),
            })?;
        }
    }
    Ok(())
}

fn rollback_target(database: &Path, attachments: Option<&Path>) {
    for path in [
        database.to_path_buf(),
        database.with_extension("db-wal"),
        database.with_extension("db-shm"),
    ] {
        let _ = fs::remove_file(path);
    }
    if let Some(attachments) = attachments {
        let _ = fs::remove_dir_all(attachments);
    }
}

/// 迁移错误。文案说明问题、影响范围与修正方式。
#[derive(Debug)]
pub enum StorageError {
    /// 目标目录不满足迁移条件。
    Target {
        /// 目标路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 在线备份失败。
    Backup {
        /// 目标数据库路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 复制附件失败。
    Copy {
        /// 目标路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 新库无法打开或完整性检查失败。
    Verify {
        /// 新库路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 位置配置写入失败。
    Config(ConfigError),
    /// 旧位置清理失败。
    Cleanup {
        /// 路径。
        path: String,
        /// 原因。
        reason: String,
    },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Target { path, reason } => {
                write!(formatter, "无法迁移到 {path}：{reason}")
            }
            StorageError::Backup { path, reason } => {
                write!(
                    formatter,
                    "备份数据库到 {path} 失败：{reason}。原位置未改动"
                )
            }
            StorageError::Copy { path, reason } => {
                write!(formatter, "复制附件到 {path} 失败：{reason}。原位置未改动")
            }
            StorageError::Verify { path, reason } => {
                write!(
                    formatter,
                    "新数据库 {path} 校验失败：{reason}。原位置未改动"
                )
            }
            StorageError::Config(error) => {
                write!(
                    formatter,
                    "已复制数据但无法记住新位置：{error}。原位置仍可用"
                )
            }
            StorageError::Cleanup { path, reason } => write!(
                formatter,
                "新位置已生效，但旧数据 {path} 未能清理：{reason}。可手动删除残留文件"
            ),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<ConfigError> for StorageError {
    fn from(error: ConfigError) -> Self {
        StorageError::Config(error)
    }
}
