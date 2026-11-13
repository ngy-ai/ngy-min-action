//! 存储对话框逻辑无头验收（T014）。
//!
//! 覆盖字节格式化、目录递归累加与展示行整理，均不依赖 GPU。

use std::collections::HashMap;
use std::path::PathBuf;

use super::{StorageInfo, collect_storage, format_bytes, storage_rows};

#[test]
fn format_bytes_uses_binary_units() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(1_048_576), "1.0 MB");
    assert_eq!(format_bytes(1_572_864), "1.5 MB");
    assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
}

#[test]
fn collect_storage_sums_files_recursively() {
    let base =
        std::env::temp_dir().join(format!("ngy-min-action-015-storage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let sub = base.join("sub");
    std::fs::create_dir_all(&sub).expect("建子目录");
    std::fs::write(base.join("a.bin"), vec![0u8; 10]).expect("写文件 a");
    std::fs::write(sub.join("b.bin"), vec![0u8; 20]).expect("写文件 b");
    // 空子目录不应影响结果
    std::fs::create_dir_all(base.join("empty")).expect("建空目录");

    let info = collect_storage(&base);
    assert_eq!(info.total_bytes, 30, "应递归累加 a(10) + b(20)");
    assert_eq!(info.data_dir, base);

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn collect_storage_handles_missing_dir() {
    let missing = std::env::temp_dir().join("ngy-min-action-015-no-such-dir-xyz");
    let _ = std::fs::remove_dir_all(&missing);
    let info = collect_storage(&missing);
    assert_eq!(info.total_bytes, 0, "目录不存在时占用为 0");
}

#[test]
fn storage_rows_show_path_and_size() {
    let info = StorageInfo {
        data_dir: PathBuf::from("/data/app"),
        total_bytes: 1536,
    };
    let map: HashMap<&str, String> = storage_rows(&info).into_iter().collect();
    assert_eq!(map.get("数据目录"), Some(&"/data/app".to_string()));
    assert_eq!(map.get("占用空间"), Some(&"1.5 KB".to_string()));
}
