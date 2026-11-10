//! 待办备注与图片附件的持久化。
//!
//! 图片文件由 `attachment` 模块负责落盘；这里只保存可展示的元数据，删除时把行
//! 交还给调用方，由调用方清理磁盘副本。

use rusqlite::{OptionalExtension, params};

use crate::model::{ActionId, ActionImage, ActionImageId, DomainError, validate_note};
use crate::store::{OpTrace, Store, now_rfc3339};

impl Store {
    /// 读取一条行动的备注。
    ///
    /// 无备注返回 `None`；行动不存在返回 [`DomainError::NotFound`]。
    ///
    /// # Errors
    ///
    /// 行动不存在或查询失败时返回 [`DomainError`]。
    pub fn action_note(&self, action: ActionId) -> Result<Option<String>, DomainError> {
        let mut trace = OpTrace::begin("action_note");
        let connection = self.guard();
        let note = connection
            .query_row(
                "SELECT note FROM actions WHERE id = ?1",
                params![action.0],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "行动",
                id: action.0,
            })?;
        trace.ok();
        Ok(note)
    }

    /// 保存或清除一条行动的备注。
    ///
    /// 入参按 [`validate_note`] 规范化：空白备注写成 `NULL`，等价于清除。
    ///
    /// # Errors
    ///
    /// 备注超长或含非法控制字符时返回 [`DomainError::InvalidValue`]；行动不存在
    /// 时返回 [`DomainError::NotFound`]。
    pub fn set_action_note(&self, action: ActionId, note: &str) -> Result<(), DomainError> {
        let mut trace = OpTrace::begin("set_action_note");
        let normalized = validate_note(note)?;
        let connection = self.guard();
        let updated = connection.execute(
            "UPDATE actions SET note = ?2 WHERE id = ?1",
            params![action.0, normalized],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound {
                entity: "行动",
                id: action.0,
            });
        }
        trace.ok();
        Ok(())
    }

    /// 列出某条行动的图片附件，按加入顺序排列。
    ///
    /// # Errors
    ///
    /// 查询失败时返回 [`DomainError::Store`]。
    pub fn action_images(&self, action: ActionId) -> Result<Vec<ActionImage>, DomainError> {
        let mut trace = OpTrace::begin("action_images");
        let connection = self.guard();
        let mut statement = connection.prepare(
            "SELECT id, action_id, file_name, original_name, byte_size, created_at
               FROM action_images WHERE action_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map(params![action.0], map_image)?;
        let mut images = Vec::new();
        for row in rows {
            images.push(row?);
        }
        trace.ok();
        Ok(images)
    }

    /// 记录一张已经落盘的图片附件。
    ///
    /// `file_name` 必须是单个文件名（不含 `/` 与 `\`），由附件模块生成；
    /// `original_name` 只用于展示，控制字符会被去掉。
    ///
    /// # Errors
    ///
    /// 文件名不合法时返回 [`DomainError::InvalidValue`]；行动不存在时返回
    /// [`DomainError::NotFound`]。
    pub fn add_action_image(
        &self,
        action: ActionId,
        file_name: &str,
        original_name: &str,
        byte_size: u64,
    ) -> Result<ActionImage, DomainError> {
        let mut trace = OpTrace::begin("add_action_image");
        if !is_single_component(file_name) {
            return Err(DomainError::InvalidValue {
                field: "图片文件名",
                expected: "不含路径分隔符的单个文件名".to_owned(),
            });
        }
        let byte_size = i64::try_from(byte_size).map_err(|_| DomainError::InvalidValue {
            field: "图片大小",
            expected: "不超过 i64 上限的字节数".to_owned(),
        })?;
        let connection = self.guard();
        let exists = connection
            .query_row(
                "SELECT 1 FROM actions WHERE id = ?1",
                params![action.0],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(DomainError::NotFound {
                entity: "行动",
                id: action.0,
            });
        }
        let original_name = sanitize_original_name(original_name);
        let created_at = now_rfc3339();
        connection.execute(
            "INSERT INTO action_images
                (action_id, file_name, original_name, byte_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![action.0, file_name, original_name, byte_size, created_at],
        )?;
        let image = ActionImage {
            id: ActionImageId(connection.last_insert_rowid()),
            action_id: action,
            file_name: file_name.to_owned(),
            original_name,
            byte_size: u64::try_from(byte_size).unwrap_or(0),
            created_at,
        };
        trace.ok();
        Ok(image)
    }

    /// 删除一张图片附件的记录并返回它，供调用方清理磁盘文件。
    ///
    /// # Errors
    ///
    /// 记录不存在时返回 [`DomainError::NotFound`]。
    pub fn delete_action_image(&self, image: ActionImageId) -> Result<ActionImage, DomainError> {
        let mut trace = OpTrace::begin("delete_action_image");
        let mut connection = self.guard();
        let transaction = connection.transaction()?;
        let image = transaction
            .query_row(
                "SELECT id, action_id, file_name, original_name, byte_size, created_at
                   FROM action_images WHERE id = ?1",
                params![image.0],
                map_image,
            )
            .optional()?
            .ok_or(DomainError::NotFound {
                entity: "图片附件",
                id: image.0,
            })?;
        transaction.execute(
            "DELETE FROM action_images WHERE id = ?1",
            params![image.id.0],
        )?;
        transaction.commit()?;
        trace.ok();
        Ok(image)
    }
}

fn is_single_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

fn sanitize_original_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|character| !character.is_control())
        .take(200)
        .collect();
    if cleaned.trim().is_empty() {
        "图片".to_owned()
    } else {
        cleaned
    }
}

fn map_image(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionImage> {
    let byte_size: i64 = row.get(4)?;
    Ok(ActionImage {
        id: ActionImageId(row.get(0)?),
        action_id: ActionId(row.get(1)?),
        file_name: row.get(2)?,
        original_name: row.get(3)?,
        byte_size: u64::try_from(byte_size).unwrap_or(0),
        created_at: row.get(5)?,
    })
}
