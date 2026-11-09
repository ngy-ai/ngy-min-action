//! 图片附件：格式与大小校验、复制进数据目录、删除副本。
//!
//! 磁盘布局固定为 `<数据目录>/attachments/<行动 ID>/<生成名>`；文件名由程序生成，
//! 用户提供的原始文件名只随数据库行保存，用于展示。

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use image::ImageFormat;

use crate::common::{IMAGE_MAX_BYTES, IMAGE_MAX_DIMENSION, MEDIA_MAX_BYTES};
use crate::model::{ActionId, TaskId, TaskMediaKind};

/// 支持的扩展名与对应的图片格式。
const SUPPORTED_EXTENSIONS: &[(&str, ImageFormat)] = &[
    ("png", ImageFormat::Png),
    ("jpg", ImageFormat::Jpeg),
    ("jpeg", ImageFormat::Jpeg),
    ("webp", ImageFormat::WebP),
    ("bmp", ImageFormat::Bmp),
];

/// 已复制进数据目录的图片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredImage {
    /// 数据目录内的文件名。
    pub file_name: String,
    /// 用户原始文件名。
    pub original_name: String,
    /// 文件字节数。
    pub byte_size: u64,
}

/// 图片附件目录。
#[derive(Debug, Clone)]
pub struct AttachmentStore {
    root: PathBuf,
}

impl AttachmentStore {
    /// 以数据目录为根构造附件目录 `<数据目录>/attachments`。
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("attachments"),
        }
    }

    /// 附件根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 某张附件的绝对路径。
    ///
    /// # Errors
    ///
    /// `file_name` 为空、为 `.` / `..` 或含路径分隔符时返回
    /// [`AttachmentError::InvalidName`]，避免读写逃出附件根目录。
    pub fn path(&self, action: ActionId, file_name: &str) -> Result<PathBuf, AttachmentError> {
        if !is_single_component(file_name) {
            return Err(AttachmentError::InvalidName {
                name: file_name.to_owned(),
            });
        }
        Ok(self.root.join(action.0.to_string()).join(file_name))
    }

    /// 校验一张本地图片并复制进附件目录。
    ///
    /// 校验顺序为：文件存在且非空、大小上限、扩展名白名单、解码成功、单边像素上限；
    /// 任一步失败都不留下副本。
    ///
    /// # Errors
    ///
    /// 文件不可读、格式不受支持、超过大小或像素上限、复制失败时返回
    /// [`AttachmentError`]，文案指出文件与修正方式。
    pub fn store(&self, action: ActionId, source: &Path) -> Result<StoredImage, AttachmentError> {
        let metadata = fs::metadata(source).map_err(|error| AttachmentError::Read {
            path: source.display().to_string(),
            reason: error.to_string(),
        })?;
        if !metadata.is_file() {
            return Err(AttachmentError::Invalid {
                path: source.display().to_string(),
                reason: "不是普通文件".to_owned(),
            });
        }
        let byte_size = metadata.len();
        if byte_size == 0 {
            return Err(AttachmentError::Invalid {
                path: source.display().to_string(),
                reason: "文件为空".to_owned(),
            });
        }
        if byte_size > IMAGE_MAX_BYTES {
            return Err(AttachmentError::Invalid {
                path: source.display().to_string(),
                reason: format!("超过单张 {} MiB 的上限", IMAGE_MAX_BYTES / 1024 / 1024),
            });
        }
        let extension = extension_of(source).ok_or_else(|| AttachmentError::Invalid {
            path: source.display().to_string(),
            reason: "缺少扩展名，仅支持 PNG、JPEG、WebP、BMP".to_owned(),
        })?;
        let expected = SUPPORTED_EXTENSIONS
            .iter()
            .find_map(|(name, format)| (*name == extension).then_some(*format))
            .ok_or_else(|| AttachmentError::Invalid {
                path: source.display().to_string(),
                reason: "扩展名不受支持，仅支持 PNG、JPEG、WebP、BMP".to_owned(),
            })?;

        let directory = self.root.join(action.0.to_string());
        fs::create_dir_all(&directory).map_err(|error| AttachmentError::Write {
            path: directory.display().to_string(),
            reason: error.to_string(),
        })?;

        let file_name = Self::generated_name(&directory, &extension);
        let destination = directory.join(&file_name);
        if let Err(error) = copy_validated_image(source, &destination, expected) {
            let _ = fs::remove_file(&destination);
            return Err(AttachmentError::Invalid {
                path: source.display().to_string(),
                reason: error,
            });
        }

        Ok(StoredImage {
            file_name,
            original_name: source
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().to_string()),
            byte_size,
        })
    }

    /// 删除一张附件。文件已不存在时视为成功。
    ///
    /// # Errors
    ///
    /// 文件名不合法或删除失败时返回 [`AttachmentError`]。
    pub fn remove(&self, action: ActionId, file_name: &str) -> Result<(), AttachmentError> {
        let path = self.path(action, file_name)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AttachmentError::Write {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    fn generated_name(directory: &Path, extension: &str) -> String {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        for attempt in 0..1000 {
            let candidate = format!("{stamp}_{attempt}.{extension}");
            if !directory.join(&candidate).exists() {
                return candidate;
            }
        }
        format!("{stamp}.{extension}")
    }
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
}

fn is_single_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

fn copy_new(source: &Path, destination: &Path) -> io::Result<u64> {
    let mut reader = fs::File::open(source)?;
    let mut writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let written = io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(written)
}

/// 图片附件错误。文案指出文件与修正方式。
#[derive(Debug)]
pub enum AttachmentError {
    /// 文件名不合法。
    InvalidName {
        /// 实际文件名。
        name: String,
    },
    /// 文件内容不合法。
    Invalid {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 读取源文件失败。
    Read {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 写入附件目录失败。
    Write {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
}

impl std::fmt::Display for AttachmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttachmentError::InvalidName { name } => {
                write!(formatter, "图片文件名「{name}」不合法，请重新选择文件")
            }
            AttachmentError::Invalid { path, reason } => {
                write!(formatter, "图片 {path} 不能添加：{reason}")
            }
            AttachmentError::Read { path, reason } => {
                write!(
                    formatter,
                    "无法读取图片 {path}：{reason}，请确认文件仍存在且可读"
                )
            }
            AttachmentError::Write { path, reason } => {
                write!(formatter, "无法复制图片到 {path}：{reason}")
            }
        }
    }
}

impl std::error::Error for AttachmentError {}

/// 任务内容媒体：音频/视频的扩展名白名单，对应其类型。
const MEDIA_EXTENSIONS: &[(&str, TaskMediaKind)] = &[
    ("mp3", TaskMediaKind::Audio),
    ("wav", TaskMediaKind::Audio),
    ("ogg", TaskMediaKind::Audio),
    ("m4a", TaskMediaKind::Audio),
    ("flac", TaskMediaKind::Audio),
    ("mp4", TaskMediaKind::Video),
    ("webm", TaskMediaKind::Video),
    ("mov", TaskMediaKind::Video),
    ("mkv", TaskMediaKind::Video),
];

/// 校验源文件是合法图片（解码 + 尺寸上限）并复制到目标，返回写入字节数。
///
/// 把报错收敛成一句可读 `reason`，由调用方套上自己的错误类型；复制失败或字节数对不上
/// 都视为失败，不留下副本。
fn copy_validated_image(
    source: &Path,
    destination: &Path,
    expected: ImageFormat,
) -> Result<u64, String> {
    let reader = image::ImageReader::open(source)
        .map_err(|error| format!("无法读取图片：{error}"))?
        .with_guessed_format()
        .map_err(|error| format!("无法识别图片格式：{error}"))?;
    let detected = reader
        .format()
        .ok_or_else(|| "无法从内容判断图片格式".to_owned())?;
    if detected != expected {
        return Err("扩展名与实际内容不一致".to_owned());
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| format!("无法读取图片尺寸：{error}"))?;
    if width > IMAGE_MAX_DIMENSION || height > IMAGE_MAX_DIMENSION {
        return Err(format!(
            "尺寸 {width}×{height} 超过 {IMAGE_MAX_DIMENSION} 像素上限"
        ));
    }
    // 完整解码一次，确保文件没有截断或损坏；解码失败不落盘。
    image::ImageReader::open(source)
        .map_err(|error| format!("无法读取图片：{error}"))?
        .with_guessed_format()
        .map_err(|error| format!("无法识别图片格式：{error}"))?
        .decode()
        .map_err(|error| format!("图片数据损坏：{error}"))?;
    copy_new(source, destination).map_err(|error| format!("复制失败：{error}"))
}

/// 已复制进数据目录的媒体附件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMedia {
    /// 数据目录内的文件名。
    pub file_name: String,
    /// 用户原始文件名。
    pub original_name: String,
    /// 文件字节数。
    pub byte_size: u64,
}

/// 任务内容媒体附件目录。
#[derive(Debug, Clone)]
pub struct MediaStore {
    root: PathBuf,
}

impl MediaStore {
    /// 以数据目录为根构造媒体目录 `<数据目录>/task_content`。
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("task_content"),
        }
    }

    /// 媒体根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 某段附件的绝对路径。
    ///
    /// # Errors
    ///
    /// `file_name` 为空、为 `.` / `..` 或含路径分隔符时返回
    /// [`MediaError::InvalidName`]，避免读写逃出媒体根目录。
    pub fn path(&self, owner: TaskId, file_name: &str) -> Result<PathBuf, MediaError> {
        if !is_single_component(file_name) {
            return Err(MediaError::InvalidName {
                name: file_name.to_owned(),
            });
        }
        Ok(self.root.join(owner.0.to_string()).join(file_name))
    }

    /// 校验一个本地媒体文件并复制进媒体目录。
    ///
    /// 图片走完整解码校验；音频/视频只做扩展名白名单与大小上限。任一步失败都不留下副本。
    ///
    /// # Errors
    ///
    /// 文件不可读、类型不受支持、超过大小上限或复制失败时返回 [`MediaError`]，文案指出
    /// 文件与修正方式。
    pub fn store(
        &self,
        owner: TaskId,
        source: &Path,
        kind: TaskMediaKind,
    ) -> Result<StoredMedia, MediaError> {
        let path_text = source.display().to_string();
        let metadata = fs::metadata(source).map_err(|error| MediaError::Read {
            path: path_text.clone(),
            reason: error.to_string(),
        })?;
        if !metadata.is_file() {
            return Err(MediaError::Invalid {
                path: path_text.clone(),
                reason: "不是普通文件".to_owned(),
            });
        }
        let byte_size = metadata.len();
        if byte_size == 0 {
            return Err(MediaError::Invalid {
                path: path_text.clone(),
                reason: "文件为空".to_owned(),
            });
        }
        let extension = extension_of(source).ok_or_else(|| MediaError::Invalid {
            path: path_text.clone(),
            reason: "缺少扩展名".to_owned(),
        })?;
        let directory = self.root.join(owner.0.to_string());
        fs::create_dir_all(&directory).map_err(|error| MediaError::Write {
            path: directory.display().to_string(),
            reason: error.to_string(),
        })?;
        let file_name = AttachmentStore::generated_name(&directory, &extension);
        let destination = directory.join(&file_name);
        match kind {
            TaskMediaKind::Image => {
                if byte_size > IMAGE_MAX_BYTES {
                    return Err(MediaError::TooLarge {
                        path: path_text.clone(),
                        limit: IMAGE_MAX_BYTES,
                    });
                }
                let expected = SUPPORTED_EXTENSIONS
                    .iter()
                    .find_map(|(name, format)| (*name == extension).then_some(*format))
                    .ok_or_else(|| MediaError::Unsupported {
                        path: path_text.clone(),
                        reason: "仅支持 PNG、JPEG、WebP、BMP".to_owned(),
                    })?;
                if let Err(reason) = copy_validated_image(source, &destination, expected) {
                    let _ = fs::remove_file(&destination);
                    return Err(MediaError::Invalid {
                        path: path_text,
                        reason,
                    });
                }
            }
            TaskMediaKind::Audio | TaskMediaKind::Video => {
                if byte_size > MEDIA_MAX_BYTES {
                    return Err(MediaError::TooLarge {
                        path: path_text.clone(),
                        limit: MEDIA_MAX_BYTES,
                    });
                }
                if MEDIA_EXTENSIONS.iter().all(|(name, _)| *name != extension) {
                    return Err(MediaError::Unsupported {
                        path: path_text.clone(),
                        reason:
                            "仅支持常见音频（mp3/wav/ogg/m4a/flac）或视频（mp4/webm/mov/mkv）格式"
                                .to_owned(),
                    });
                }
                if let Err(error) = copy_new(source, &destination) {
                    let _ = fs::remove_file(&destination);
                    return Err(MediaError::Write {
                        path: destination.display().to_string(),
                        reason: error.to_string(),
                    });
                }
            }
        }
        Ok(StoredMedia {
            file_name,
            original_name: source
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().to_string()),
            byte_size,
        })
    }

    /// 删除一段附件。文件已不存在时视为成功。
    ///
    /// # Errors
    ///
    /// 文件名不合法或删除失败时返回 [`MediaError`]。
    pub fn remove(&self, owner: TaskId, file_name: &str) -> Result<(), MediaError> {
        let path = self.path(owner, file_name)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(MediaError::Write {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }
}

/// 媒体附件错误。文案指出文件与修正方式。
#[derive(Debug)]
pub enum MediaError {
    /// 文件名不合法。
    InvalidName {
        /// 实际文件名。
        name: String,
    },
    /// 文件内容不合法。
    Invalid {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 读取源文件失败。
    Read {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 写入媒体目录失败。
    Write {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
    /// 超过大小上限。
    TooLarge {
        /// 文件路径。
        path: String,
        /// 允许的最大字节数。
        limit: u64,
    },
    /// 扩展名不在白名单。
    Unsupported {
        /// 文件路径。
        path: String,
        /// 原因。
        reason: String,
    },
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaError::InvalidName { name } => {
                write!(formatter, "媒体文件名「{name}」不合法，请重新选择文件")
            }
            MediaError::Invalid { path, reason } => {
                write!(formatter, "媒体 {path} 不能添加：{reason}")
            }
            MediaError::Read { path, reason } => {
                write!(
                    formatter,
                    "无法读取媒体 {path}：{reason}，请确认文件仍存在且可读"
                )
            }
            MediaError::Write { path, reason } => {
                write!(formatter, "无法复制媒体到 {path}：{reason}")
            }
            MediaError::TooLarge { path, limit } => {
                write!(
                    formatter,
                    "媒体 {path} 超过 {} MiB 的上限",
                    limit / 1024 / 1024
                )
            }
            MediaError::Unsupported { path, reason } => {
                write!(formatter, "媒体 {path} 不支持：{reason}")
            }
        }
    }
}

impl std::error::Error for MediaError {}
