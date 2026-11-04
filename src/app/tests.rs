//! 界面层验收：按 `specs/004-edit-and-delete/quickstart.md` 的五个场景驱动真实状态机。
//!
//! 这里不模拟鼠标，而是直接调用界面行会触发的同一套方法（打开弹窗、提交、
//! 请求删除、确认、取消），断言数据与弹窗状态；另有一条 headless 渲染用例，
//! 确保三个视图与弹窗在无窗口环境下不 panic。

use std::sync::Arc;
use std::sync::mpsc;

use chrono::NaiveDate;
use eframe::egui;

use super::{FrequencyMode, MinActionApp, PendingDelete, RenameTarget, TickEvent};
use crate::clock::{Clock, FixedClock};
use crate::model::{ActionStatus, CategoryId, Frequency, TaskDraft, TaskId, TaskStatus};
use crate::store::Store;

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

#[test]
fn scenario_1_renaming_a_goal_updates_the_tree() {
    let mut app = open_app("rename-goal");
    let goal = app.store.create_goal("学习英语", None).expect("创建目标");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Goal(goal), "学习英语");
    assert_eq!(app.draft_rename_name, "学习英语", "打开时应预填当前名称");
    assert!(app.rename_select_all, "打开时应请求全选输入框内容");

    app.draft_rename_name = "英语".to_owned();
    app.submit_rename();

    assert!(app.rename_target.is_none(), "成功后应关闭弹窗");
    assert_eq!(app.goals[0].name, "英语", "左侧树应显示新名称");
    assert!(app.error.is_none(), "成功时不应有错误提示");
}

#[test]
fn scenario_1_renaming_a_goal_to_an_existing_name_is_rejected() {
    let mut app = open_app("rename-goal-duplicate");
    app.store.create_goal("学习英语", None).expect("创建目标");
    let japanese = app.store.create_goal("日语", None).expect("创建目标");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Goal(japanese), "日语");
    app.draft_rename_name = "学习英语".to_owned();
    app.submit_rename();

    assert_eq!(
        app.rename_target,
        Some(RenameTarget::Goal(japanese)),
        "失败时弹窗应保留"
    );
    assert_eq!(app.draft_rename_name, "学习英语", "失败时输入应保留");
    let message = app.error.as_deref().unwrap_or_default();
    assert!(message.contains("必须唯一"), "应说明重名：{message}");
    let names: Vec<&str> = app.goals.iter().map(|goal| goal.name.as_str()).collect();
    assert_eq!(names, ["学习英语", "日语"], "数据不应变化");
}

#[test]
fn scenario_2_renaming_a_category_keeps_its_tasks() {
    let mut app = open_app("rename-category");
    let goal = app.store.create_goal("学习英语", None).expect("创建目标");
    let grammar = app.store.create_category(goal, "语法").expect("创建分类");
    let task = app
        .store
        .create_task(grammar, draft("时态"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Category(grammar), "语法");
    app.draft_rename_name = "语法练习".to_owned();
    app.submit_rename();

    assert!(app.rename_target.is_none(), "成功后应关闭弹窗");
    let renamed = app
        .categories
        .iter()
        .find(|item| item.id == grammar)
        .map(|item| item.name.as_str());
    assert_eq!(renamed, Some("语法练习"), "分类行应显示新名称");
    assert_eq!(name_of(&app, task), "时态", "其下任务不受影响");
}

#[test]
fn scenario_2_renaming_a_category_to_a_sibling_name_is_rejected() {
    let mut app = open_app("rename-category-duplicate");
    let goal = app.store.create_goal("学习英语", None).expect("创建目标");
    app.store.create_category(goal, "口语").expect("创建分类");
    let grammar = app.store.create_category(goal, "语法").expect("创建分类");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Category(grammar), "语法");
    app.draft_rename_name = "口语".to_owned();
    app.submit_rename();

    assert_eq!(
        app.rename_target,
        Some(RenameTarget::Category(grammar)),
        "失败时弹窗应保留"
    );
    assert!(
        app.error
            .as_deref()
            .unwrap_or_default()
            .contains("必须唯一"),
        "应说明同目标下重名"
    );
    let names: Vec<&str> = app
        .all_categories
        .iter()
        .map(|category| category.name.as_str())
        .collect();
    assert_eq!(names, ["口语", "语法"], "数据不应变化");
}

#[test]
fn scenario_3_renaming_a_task_updates_lists_and_keeps_progress() {
    let mut app = open_app("rename-task");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let frequency = app.tasks[0].frequency;
    let progress_before = app.task_progress.len();

    app.open_rename_dialog(RenameTarget::Task(task), "连读");
    assert_eq!(app.draft_rename_name, "连读", "打开时应预填当前名称");
    app.draft_rename_name = "连读练习".to_owned();
    app.submit_rename();

    assert!(app.rename_target.is_none(), "成功后应关闭弹窗");
    assert_eq!(app.tasks[0].name, "连读练习", "任务行应显示新名称");
    assert_eq!(name_of(&app, task), "连读练习", "今日清单解析名称同步");
    assert_eq!(app.tasks[0].frequency, frequency, "频率不应变化");
    assert_eq!(app.task_progress.len(), progress_before, "复习进度不应变化");
}

#[test]
fn scenario_3_duplicate_task_names_are_allowed() {
    let mut app = open_app("rename-task-duplicate");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    let second = app
        .store
        .create_task(category, draft("连读练习"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Task(second), "连读练习");
    app.draft_rename_name = "连读".to_owned();
    app.submit_rename();

    assert!(
        app.rename_target.is_none(),
        "任务名不校验唯一性，应直接成功"
    );
    let names: Vec<&str> = app.tasks.iter().map(|task| task.name.as_str()).collect();
    assert_eq!(names, ["连读", "连读"], "允许同名任务");
}

#[test]
fn scenario_4_deleting_a_task_reports_open_actions_then_expires_them() {
    let mut app = open_app("delete-task");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let open_actions = app.store.task_open_actions(task).expect("读取未完成行动");
    assert_eq!(
        open_actions,
        app.actions
            .iter()
            .filter(|item| item.task_id == task)
            .count(),
        "当天每个行动都应计入"
    );
    assert!(open_actions > 0, "新建任务应生成今天的行动");
    let pending_before = app
        .actions
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();

    app.request_delete(PendingDelete::Task(task));
    assert_eq!(app.pending_delete, Some(PendingDelete::Task(task)));
    assert_eq!(
        app.delete_impact.map(|impact| impact.pending_actions),
        Some(open_actions),
        "确认框应说明将作废的行动数"
    );
    assert!(
        app.tasks.iter().any(|item| item.id == task),
        "此时任务尚未删除"
    );

    app.confirm_delete();

    assert!(app.pending_delete.is_none(), "确认后应关闭弹窗");
    assert!(
        !app.tasks.iter().any(|item| item.id == task),
        "任务应从列表消失"
    );
    assert!(
        app.actions
            .iter()
            .filter(|item| item.task_id == task)
            .all(|item| item.status == ActionStatus::Expired),
        "未完成行动应转为已过期"
    );
    let pending_after = app
        .actions
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();
    assert_eq!(
        pending_after,
        pending_before - open_actions,
        "待完成数量应相应减少"
    );
}

#[test]
fn scenario_4_cancelling_keeps_the_task_and_actions() {
    let mut app = open_app("delete-task-cancel");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let actions_before = app.actions.len();

    app.request_delete(PendingDelete::Task(task));
    app.cancel_delete();

    assert!(app.pending_delete.is_none(), "取消后应关闭弹窗");
    assert!(app.delete_impact.is_none(), "取消后应清空影响范围");
    assert!(
        app.tasks.iter().any(|item| item.id == task),
        "任务应仍在列表中"
    );
    assert_eq!(app.actions.len(), actions_before, "行动不应变化");
    assert!(
        app.actions
            .iter()
            .all(|item| item.status != ActionStatus::Expired),
        "不应有行动被置为过期"
    );
}

#[test]
fn scenario_5_renaming_to_the_same_name_is_allowed() {
    let mut app = open_app("rename-same-name");
    let goal = app.store.create_goal("学习英语", None).expect("创建目标");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Goal(goal), "学习英语");
    app.submit_rename();

    assert!(app.rename_target.is_none(), "改名为原名不应报错");
    assert!(app.error.is_none(), "改名为原名不应有错误提示");
    assert_eq!(app.goals[0].name, "学习英语");
}

#[test]
fn scenario_5_switching_category_keeps_the_dialog_target() {
    let mut app = open_app("rename-dialog-target");
    let goal = app.store.create_goal("学习英语", None).expect("创建目标");
    let spoken = app.store.create_category(goal, "口语").expect("创建分类");
    let grammar = app.store.create_category(goal, "语法").expect("创建分类");
    let task = app
        .store
        .create_task(spoken, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_rename_dialog(RenameTarget::Task(task), "连读");
    app.selected_category = Some(grammar);
    app.refresh_scope();
    app.draft_rename_name = "连读练习".to_owned();
    app.submit_rename();

    assert_eq!(name_of(&app, task), "连读练习", "改的仍是弹窗打开时的任务");
}

/// 跑一帧 headless 渲染，并丢弃纹理增量（无窗口时没人消费它）。
fn render_one_frame(ctx: &egui::Context, mut draw: impl FnMut(&mut egui::Ui)) {
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
    output.textures_delta.clear();
}

#[test]
fn dialogs_and_views_render_without_panicking() {
    let mut app = open_app("render");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let ctx = egui::Context::default();
    render_one_frame(&ctx, |ui| app.ui_main_view(ui));
    render_one_frame(&ctx, |ui| app.ui_today_view(ui));

    app.open_rename_dialog(RenameTarget::Task(task), "连读");
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_rename_dialog(&ctx);
    });

    assert!(!app.rename_select_all, "渲染后应清除待全选标记");
    let edit_id = egui::Id::unique("dialog-rename-name");
    let state =
        egui::widgets::text_edit::TextEditState::load(&ctx, edit_id).expect("输入框应已写入状态");
    let range = state
        .cursor
        .char_range()
        .expect("打开弹窗后应全选输入框内容");
    assert_eq!(
        range.primary.index,
        egui::text::CharIndex(2),
        "「连读」两个字符都应被选中"
    );
    assert_eq!(
        range.secondary.index,
        egui::text::CharIndex(0),
        "选区应从开头起算"
    );

    app.request_delete(PendingDelete::Task(task));
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_delete_dialog(&ctx);
    });

    let current = app.tasks[0].frequency;
    app.open_frequency_dialog(task, current);
    render_one_frame(&ctx, |ui| {
        let ctx = ui.ctx().clone();
        app.ui_frequency_dialog(&ctx);
    });
}

#[test]
fn editing_the_frequency_updates_only_the_task_row() {
    let mut app = open_app("frequency-edit");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    let actions_before = app.actions.len();
    let progress_before = app.task_progress.len();

    app.open_frequency_dialog(task, Frequency::Daily);
    assert_eq!(
        app.draft_frequency_mode,
        FrequencyMode::Daily,
        "打开时应预选当前模式"
    );
    app.draft_frequency_mode = FrequencyMode::EveryNDays;
    app.draft_frequency_interval = "3".to_owned();
    app.submit_frequency();

    assert!(app.frequency_target.is_none(), "成功后应关闭弹窗");
    assert_eq!(
        app.tasks[0].frequency,
        Frequency::EveryNDays(3),
        "任务行应显示新频率"
    );
    assert_eq!(app.tasks[0].name, "连读", "名称不应变化");
    assert_eq!(app.tasks[0].status, TaskStatus::Active, "状态不应变化");
    assert_eq!(app.task_progress.len(), progress_before, "复习进度不应变化");
    assert_eq!(app.actions.len(), actions_before, "已生成的行动不应变化");

    // 再保存一次相同的值，不应触发任何错误。
    app.open_frequency_dialog(task, Frequency::EveryNDays(3));
    app.submit_frequency();
    assert!(app.frequency_target.is_none(), "改成当前值也应成功");
    assert!(app.error.is_none(), "改成当前值不应报错");
}

#[test]
fn invalid_frequency_parameters_keep_the_dialog_open() {
    let mut app = open_app("frequency-invalid");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    app.open_frequency_dialog(task, Frequency::Daily);
    app.draft_frequency_mode = FrequencyMode::EveryNDays;
    app.draft_frequency_interval = "0".to_owned();
    app.submit_frequency();

    assert_eq!(app.frequency_target, Some(task), "失败时弹窗应保留");
    assert_eq!(app.draft_frequency_interval, "0", "失败时输入应保留");
    let message = app.error.as_deref().unwrap_or_default();
    assert!(message.contains("频率间隔"), "应指出是频率间隔：{message}");
    assert_eq!(
        app.tasks[0].frequency,
        Frequency::Daily,
        "被拒后数据不应变化"
    );
}

#[test]
fn paused_tasks_accept_frequency_changes() {
    let mut app = open_app("frequency-paused");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");
    app.reload().expect("刷新界面数据");

    app.open_frequency_dialog(task, Frequency::Daily);
    app.draft_frequency_mode = FrequencyMode::WeeklyN;
    app.draft_frequency_interval = "3".to_owned();
    app.submit_frequency();

    assert_eq!(app.tasks[0].frequency, Frequency::WeeklyN(3));
    assert_eq!(app.tasks[0].status, TaskStatus::Paused, "应仍为暂停");
}

#[test]
fn raising_the_frequency_takes_effect_on_the_next_day() {
    let (mut app, clock) = open_app_with_clock("frequency-cross-day");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    assert_eq!(app.actions.len(), 1, "每天任务开局就有一条行动");

    app.open_frequency_dialog(task, Frequency::WeeklyN(1));
    assert_eq!(
        app.draft_frequency_mode,
        FrequencyMode::WeeklyN,
        "应预选当前模式"
    );
    assert_eq!(app.draft_frequency_interval, "1", "应预填当前参数");
    app.submit_frequency();
    assert_eq!(app.tasks[0].frequency, Frequency::WeeklyN(1));
    assert_eq!(app.actions.len(), 1, "今天的行动不应被改动");

    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    assert_eq!(app.actions.len(), 0, "每周 1 次的间隔未到，次日不应生成");

    app.open_frequency_dialog(task, Frequency::WeeklyN(1));
    app.draft_frequency_mode = FrequencyMode::Daily;
    app.submit_frequency();
    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    assert_eq!(app.actions.len(), 1, "改回每天后次日应生成行动");
}

#[test]
fn deleting_a_task_without_open_actions_reports_zero() {
    let mut app = open_app("delete-task-no-open");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let actions: Vec<_> = app.actions.iter().map(|item| item.id).collect();
    assert!(!actions.is_empty(), "新建任务应生成今天的行动");
    for action in actions {
        app.store
            .settle_action(action, ActionStatus::Done)
            .expect("完成行动");
    }
    app.reload().expect("刷新界面数据");
    assert_eq!(
        app.store.task_open_actions(task).expect("计数"),
        0,
        "全部结算后不应再有未完成行动"
    );

    app.request_delete(PendingDelete::Task(task));
    assert_eq!(
        app.delete_impact.map(|impact| impact.pending_actions),
        Some(0),
        "确认框应说明将作废 0 条行动"
    );

    app.confirm_delete();
    assert!(
        !app.tasks.iter().any(|item| item.id == task),
        "作废 0 条时也应能正常删除"
    );
}

#[test]
fn completed_actions_survive_task_deletion() {
    let mut app = open_app("delete-task-keeps-done");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let action = app.actions[0].id;
    app.store
        .settle_action(action, ActionStatus::Done)
        .expect("完成行动");
    app.reload().expect("刷新界面数据");

    app.request_delete(PendingDelete::Task(task));
    app.confirm_delete();

    let kept = app
        .actions
        .iter()
        .find(|item| item.id == action)
        .map(|item| item.status);
    assert_eq!(kept, Some(ActionStatus::Done), "已完成行动的历史记录应保留");
}

#[test]
fn resetting_review_progress_requires_confirmation() {
    let (mut app, clock) = open_app_with_clock("reset-review");
    let category = seed_category(&mut app, "学习英语", "口语");
    let task = app
        .store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let first = app.actions[0].id;
    app.store
        .settle_action(first, ActionStatus::Done)
        .expect("完成第一次");
    clock.advance_days(1);
    app.today = clock.today();
    app.reload().expect("跨日刷新");
    let second = app.actions[0].id;
    app.store
        .settle_action(second, ActionStatus::Done)
        .expect("完成第二次");
    app.reload().expect("刷新界面数据");

    let level = |app: &MinActionApp| {
        app.task_progress
            .iter()
            .find(|item| item.task_id == task)
            .map(|item| item.level)
    };
    assert_eq!(level(&app), Some(2), "前置条件：进度应为第 2 级");

    app.request_reset(task);
    assert_eq!(app.reset_target, Some(task), "应打开确认框");
    assert_eq!(level(&app), Some(2), "确认前不应改动数据");

    app.cancel_reset();
    assert!(app.reset_target.is_none(), "取消后应关闭");
    assert_eq!(level(&app), Some(2), "取消后进度不变");

    app.request_reset(task);
    app.confirm_reset();
    assert!(app.reset_target.is_none(), "确认后应关闭");
    assert_eq!(level(&app), Some(0), "确认后应归零");
    assert_eq!(app.tasks[0].status, TaskStatus::Active, "任务状态不变");
}

#[test]
fn resetting_a_missing_task_keeps_the_dialog_open() {
    let mut app = open_app("reset-review-missing");
    let ghost = TaskId(9999);

    app.request_reset(ghost);
    assert_eq!(app.reset_target, Some(ghost), "应打开确认框");

    app.confirm_reset();
    assert_eq!(app.reset_target, Some(ghost), "失败时弹窗应保留");
    let message = app.error.as_deref().unwrap_or_default();
    assert!(message.contains("不存在"), "应说明任务不存在：{message}");
}
