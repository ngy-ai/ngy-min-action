//! 配置窗口的无头验收：存储迁移接线与左侧页签清单。

use std::path::PathBuf;

use crate::app::theme_label;
use crate::config::Bootstrap;
use crate::model::ThemeChoice;
use crate::store::Store;

use super::{ConfigTab, migrate_store};

/// 左侧页签应覆盖每日上限、存储地址与主题，顺序固定。
#[test]
fn tabs_cover_all_settings() {
    let labels: Vec<&str> = ConfigTab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(
        labels,
        vec!["每日上限", "存储地址", "主题"],
        "配置窗口左侧应有三项设置"
    );
}

/// 主题选项文案与设置项一一对应，不复用持久化取值。
#[test]
fn theme_options_are_readable() {
    assert_eq!(theme_label(ThemeChoice::System), "跟随系统");
    assert_eq!(theme_label(ThemeChoice::Light), "浅色");
    assert_eq!(theme_label(ThemeChoice::Dark), "深色");
}

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
