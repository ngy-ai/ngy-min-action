//! 004：目标、分类、任务改名的用例。

use super::*;

use crate::app::RenameTarget;

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
