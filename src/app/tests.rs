//! 界面层验收：按 quickstart 的场景驱动真实状态机。
//!
//! 用例按对象分文件：`rename`、`delete`、`frequency`、`reset`、`render`。
//! 这里不模拟鼠标，而是直接调用界面行会触发的同一套方法（打开弹窗、提交、
//! 请求删除、确认、取消），断言数据与弹窗状态。

use std::sync::Arc;
use std::sync::mpsc;

use chrono::NaiveDate;

use super::{MinActionApp, TickEvent};
use crate::clock::FixedClock;
use crate::model::{CategoryId, Frequency, TaskDraft, TaskId};
use crate::store::Store;

mod context_menu;
mod delete;
mod estimate;
mod frequency;
mod icons;
mod rename;
mod render;
mod reset;
mod stats;
mod theme;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 4).expect("测试日期必须合法")
}

fn open_app(tag: &str) -> MinActionApp {
    open_app_with_clock(tag).0
}

/// 同 [`open_app`]，额外把时钟交出来，供跨日用例推进日期。
fn open_app_with_clock(tag: &str) -> (MinActionApp, Arc<FixedClock>) {
    let dir = std::env::temp_dir().join(format!(
        "ngy-min-action-app-test-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::open(&dir.join("data.db")).expect("应能创建临时数据库");
    let clock = Arc::new(FixedClock::new(today()));
    let (_sender, receiver) = mpsc::channel::<TickEvent>();
    let app = MinActionApp::new(store, clock.clone(), receiver).expect("应能构建应用");
    (app, clock)
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
    }
}

/// 建一个目标 + 分类，返回分类 ID，并刷新界面快照。
fn seed_category(app: &mut MinActionApp, goal: &str, category: &str) -> CategoryId {
    let goal = app.store.create_goal(goal, None).expect("创建目标");
    let category = app.store.create_category(goal, category).expect("创建分类");
    app.reload().expect("刷新界面数据");
    category
}

fn name_of(app: &MinActionApp, task: TaskId) -> String {
    app.all_tasks
        .iter()
        .find(|item| item.id == task)
        .map(|item| item.name.clone())
        .unwrap_or_default()
}
