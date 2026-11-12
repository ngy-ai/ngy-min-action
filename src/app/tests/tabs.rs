//! 主区页签与结构树计数的 headless 验收。

use super::*;

use crate::app::TodoTab;
use crate::model::GoalId;
use eframe::egui;

/// 跑一帧 headless 渲染，并丢弃纹理增量（无窗口时没人消费它）。
fn render_one_frame(ctx: &egui::Context, mut draw: impl FnMut(&mut egui::Ui)) {
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
    output.textures_delta.clear();
}

#[test]
fn three_tabs_switch_and_render_without_panicking() {
    let mut app = open_app("main-tabs");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    for tab in [TodoTab::Tasks, TodoTab::Today, TodoTab::History] {
        app.switch_tab(tab);
        assert_eq!(app.tab, tab, "页签应切换到 {tab:?}");
        let ctx = egui::Context::default();
        render_one_frame(&ctx, |ui| app.ui_tabs(ui));
    }
}

#[test]
fn tree_counts_tasks_per_category_and_goal() {
    let mut app = open_app("tree-counts");
    let goal: GoalId = app.store.create_goal("AI", None).expect("创建目标");
    let spoken = app
        .store
        .create_category(goal, "LLM")
        .expect("创建分类 LLM");
    let agent = app
        .store
        .create_category(goal, "Agent")
        .expect("创建分类 Agent");
    app.store.create_task(spoken, draft("微调")).expect("任务1");
    app.store.create_task(spoken, draft("评测")).expect("任务2");
    app.store
        .create_task(agent, draft("工具调用"))
        .expect("任务3");
    app.reload().expect("刷新界面数据");

    assert_eq!(app.task_counts.get(&spoken), Some(&2), "分类计数应含其任务");
    assert_eq!(app.task_counts.get(&agent), Some(&1));
    assert_eq!(app.goal_task_count(goal), 3, "目标计数应汇总其全部分类");
}

#[test]
fn counts_refresh_after_deleting_a_task() {
    let mut app = open_app("tree-counts-delete");
    let category = seed_category(&mut app, "AI", "LLM");
    let task = app
        .store
        .create_task(category, draft("微调"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");
    assert_eq!(app.task_counts.get(&category), Some(&1));

    app.store.delete_task(task).expect("删除任务");
    app.reload().expect("刷新界面数据");
    assert_eq!(
        app.task_counts.get(&category).copied().unwrap_or(0),
        0,
        "删除后计数应归零（无任务分类不留在计数表里）"
    );
}
