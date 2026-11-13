//! 备注对话框错误文案无头验收（T012）。
//!
//! 对话框的核心是「保存失败时给出可读错误且不关闭」——这里直接验证错误文案纯函数，无需 GPU。

use super::format_note_error;
use crate::model::DomainError;

#[test]
fn error_text_is_readable() {
    let err = DomainError::InvalidValue {
        field: "备注",
        expected: "不超过 2000 个字符".to_string(),
    };
    let text = format_note_error(&err);
    assert!(text.starts_with("保存失败："), "应带「保存失败」前缀");
    assert!(text.contains("备注"), "应指明是备注字段");
    assert!(text.contains("2000"), "应提示长度上限");
}
