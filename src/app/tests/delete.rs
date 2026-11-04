//! 004：任务删除的用例，含作废行动数与已完成记录的边界。

use super::*;

use crate::app::PendingDelete;
use crate::model::ActionStatus;

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
