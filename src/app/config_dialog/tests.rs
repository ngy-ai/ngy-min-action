//! 配置窗口的存储迁移接线验收（数据层已有独立测试，这里验证 UI 侧调用结果）。

use std::path::PathBuf;

use crate::config::Bootstrap;
use crate::store::Store;

use super::migrate_store;

/// 为每次测试准备隔离的临时根，避免目录互相污染。
fn temp_base(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("ngy-min-action-017-config-{tag}"));
    let _ = std::fs::remove_dir_all(&base);
    base
}

#[test]
fn migrate_store_copies_data_and_points_config_to_new_dir() {
    let base = temp_base("migrate");
    let old_dir = base.join("old");
    let boot_dir = base.join("boot");
    let new_dir = base.join("new");
    std::fs::create_dir_all(&old_dir).expect("建旧目录");

    let store = Store::open(&old_dir.join("data.db")).expect("打开旧库");
    store.create_goal("G", None).expect("写入目标");
    let bootstrap = Bootstrap::new(boot_dir);

    let (new_store, old) = migrate_store(&store, &new_dir, &bootstrap).expect("迁移应成功");

    assert_eq!(old, old_dir, "应返回旧目录供确认后清理");
    assert_eq!(new_store.data_dir(), new_dir, "新连接应指向新目录");
    assert_eq!(bootstrap.data_dir(), new_dir, "位置配置应指向新目录");
    assert_eq!(
        new_store.goals().expect("读目标").len(),
        1,
        "数据应随迁移保留"
    );
}
