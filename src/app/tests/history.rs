//! 历史待办：分页、只读备注与结算后的刷新。

use super::*;

use crate::app::TodoTab;
use crate::model::{ActionId, ActionStatus};
use chrono::Duration;

fn seed_history(app: &mut MinActionApp) -> Vec<ActionId> {
    let category = seed_category(app, "健身", "力量");
    app.store
        .create_task(category, draft("俯卧撑"))
        .expect("创建任务");
    let today = app.today;
    let mut ids = Vec::new();
    for offset in [2, 1] {
        let actions = app
            .store
            .generate_day(today - Duration::days(offset), 5)
            .expect("生成历史行动");
        ids.extend(actions.iter().map(|action| action.id));
    }
    app.reload().expect("刷新界面数据");
    ids
}

#[test]
fn history_lists_past_days_newest_first() {
    let mut app = open_app("history-order");
    seed_history(&mut app);

    app.switch_tab(TodoTab::History);
    assert!(app.history.loaded);
    assert_eq!(app.history.actions.len(), 2);
    assert_eq!(
        app.history.actions[0].on_date,
        app.today - Duration::days(1)
    );
    assert_eq!(
        app.history.actions[1].on_date,
        app.today - Duration::days(2)
    );
    assert!(!app.history.has_more);
    assert!(app.history.error.is_none());
}

#[test]
fn history_note_opens_read_only() {
    let mut app = open_app("history-note");
    seed_history(&mut app);
    app.switch_tab(TodoTab::History);
    let action = app.history.actions[0].id;
    app.store
        .set_action_note(action, "当时做完记了一笔")
        .expect("写备注");
    app.reset_history();
    app.ensure_history();

    app.open_note(action, true);
    let editor = app.note.as_ref().expect("备注窗口应打开");
    assert!(editor.read_only, "历史备注只读");
    assert_eq!(editor.text, "当时做完记了一笔");
}

#[test]
fn settling_from_history_refreshes_status() {
    let mut app = open_app("history-settle");
    seed_history(&mut app);
    app.switch_tab(TodoTab::History);
    let action = app.history.actions[0].id;

    app.apply(|app| app.store.settle_action(action, ActionStatus::Done));
    app.reset_history();
    app.ensure_history();

    let row = app
        .history
        .actions
        .iter()
        .find(|item| item.id == action)
        .expect("历史中仍有该行");
    assert_eq!(row.status, ActionStatus::Done);
}
