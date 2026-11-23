//! 根视图数据桥接的无头验收（T005）。
//!
//! 不依赖 GPU/窗口，直接构造 `MinActionApp` 并断言 `reload()` 从 `Store` 正确载入目标、分类、
//! 任务与分类计数。后续阶段的结构树与三页签都依赖这份状态。

use std::sync::Arc;

use chrono::NaiveDate;

use crate::app::{MinActionApp, TodoTab};
use crate::clock::SystemClock;
use crate::model::{ActionStatus, Frequency, TaskDraft, ThemeChoice};
use crate::store::Store;

/// 在临时目录打开一个空库，便于测试构造数据。
fn open_temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("ngy-min-action-015-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(&dir.join("data.db")).expect("应能创建临时数据库")
}

/// `reload()` 应载入目标、分类、任务，并聚合出分类任务计数。
#[test]
fn reload_loads_goals_categories_tasks_and_counts() {
    let store = open_temp_store("bridge");
    let goal = store.create_goal("AI", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(category, draft).expect("创建任务");

    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert_eq!(app.all_goals.len(), 1, "应载入 1 个目标");
    assert_eq!(app.all_categories.len(), 1, "应载入 1 个分类");
    assert_eq!(app.all_tasks.len(), 1, "应载入 1 个任务");
    assert_eq!(
        app.task_counts.get(&category),
        Some(&1),
        "分类任务计数应为 1"
    );
    assert_eq!(app.selected_goal, None, "默认应处于「全部」层级");
    assert_eq!(app.selected_category, None, "默认不选中任何分类");
    assert_eq!(app.goal_task_count(goal), 1, "目标任务汇总应为 1");
}

/// 选中目标后进入「目标」层级：展示该目标下所有任务，不自动选中分类。
#[test]
fn select_goal_shows_all_tasks_of_goal() {
    let store = open_temp_store("select");
    let goal1 = store.create_goal("AI", None).expect("目标1");
    let cat1 = store.create_category(goal1, "口语").expect("分类1");
    let cat1b = store.create_category(goal1, "听力").expect("分类1b");
    let goal2 = store.create_goal("健康", None).expect("目标2");
    let cat2 = store.create_category(goal2, "运动").expect("分类2");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat1, draft.clone()).expect("任务1");
    store.create_task(cat1b, draft.clone()).expect("任务2");
    store.create_task(cat2, draft).expect("任务3");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    app.select_goal(goal1);
    assert_eq!(app.selected_goal, Some(goal1), "应选中目标1");
    assert_eq!(app.selected_category, None, "目标层级不应自动选中分类");
    assert_eq!(app.filtered_tasks().len(), 2, "应展示目标1下全部任务");

    app.select_category(cat1);
    assert_eq!(app.selected_category, Some(cat1), "应选中分类1");
    assert_eq!(app.selected_goal, Some(goal1), "所属目标应同步为目标1");
    assert_eq!(app.filtered_tasks().len(), 1, "分类层级只展示该分类任务");
}

/// 切换页签应更新当前 tab 状态。
#[test]
fn switch_tab_updates_state() {
    let store = open_temp_store("tab");
    let _goal = store.create_goal("AI", None).expect("目标");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert_eq!(app.tab, TodoTab::Tasks, "默认应为任务页");
    app.switch_tab(TodoTab::History).expect("切换到历史页");
    assert_eq!(app.tab, TodoTab::History, "应切换到历史页");
    app.switch_tab(TodoTab::Stats).expect("切换到统计页");
    assert_eq!(app.tab, TodoTab::Stats, "应切换到统计页");
}

/// 统计页与任务 / 今日待办 / 历史待办同级：切换过去时重新计算统计，展示最新数字。
#[test]
fn switching_to_stats_tab_refreshes_review_stats() {
    let store = open_temp_store("stats-tab");
    let goal = store.create_goal("AI", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(category, draft).expect("创建任务");

    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");
    assert!(app.review_stats.is_some(), "reload 后应算出统计快照");

    let action_id = app.today_actions.first().expect("今日应有行动").id;
    app.settle_today_action(action_id, ActionStatus::Done)
        .expect("结算完成");
    app.switch_tab(TodoTab::Stats).expect("切换到统计页");

    let stats = app.review_stats.as_ref().expect("统计页应有统计快照");
    assert_eq!(stats.tally.done, 1, "切到统计页时应看到刚完成的行动");
}

/// 「编辑」下拉：点击展开，再点收起，显式关闭后为空。
#[test]
fn menu_dropdown_toggles_and_closes() {
    let store = open_temp_store("menu");
    let _goal = store.create_goal("AI", None).expect("目标");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert!(!app.menu_open, "初始不应展开下拉");
    app.toggle_menu();
    assert!(app.menu_open, "点击后应展开下拉");
    app.toggle_menu();
    assert!(!app.menu_open, "再次点击同一项应收起");
    app.toggle_menu();
    app.close_menu();
    assert!(!app.menu_open, "关闭后应无展开项");
}

/// 删除目标的影响范围应汇总其下分类与任务数。
#[test]
fn goal_deletion_impact_sums_categories_and_tasks() {
    let store = open_temp_store("impact-goal");
    let goal = store.create_goal("G", None).expect("目标");
    let cat1 = store.create_category(goal, "C1").expect("分类1");
    let cat2 = store.create_category(goal, "C2").expect("分类2");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat1, draft.clone());
    let _ = store.create_task(cat2, draft);
    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");
    let impact = app.goal_deletion_impact(goal);
    assert_eq!(impact.categories, 2, "应波及 2 个分类");
    assert_eq!(impact.tasks, 2, "应波及 2 条任务");
}

/// 删除分类的影响范围应只含其下任务数。
#[test]
fn category_deletion_impact_counts_tasks() {
    let store = open_temp_store("impact-cat");
    let goal = store.create_goal("G", None).expect("目标");
    let cat = store.create_category(goal, "C").expect("分类");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    let _ = store.create_task(cat, draft);
    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");
    let impact = app.category_deletion_impact(cat);
    assert_eq!(impact.categories, 0, "分类删除不影响其他分类");
    assert_eq!(impact.tasks, 1, "应波及 1 条任务");
}

/// 「全部 / 目标 / 分类」三级选择应逐级收窄任务列表。
#[test]
fn three_level_selection_filters_tasks() {
    let store = open_temp_store("levels");
    let g1 = store.create_goal("G1", None).expect("目标1");
    let c1 = store.create_category(g1, "C1").expect("分类1");
    let c2 = store.create_category(g1, "C2").expect("分类2");
    let g2 = store.create_goal("G2", None).expect("目标2");
    let c3 = store.create_category(g2, "C3").expect("分类3");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(c1, draft.clone()).expect("任务1");
    store.create_task(c2, draft.clone()).expect("任务2");
    store.create_task(c3, draft).expect("任务3");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert_eq!(app.filtered_tasks().len(), 3, "默认「全部」应展示所有任务");
    app.select_goal(g2);
    assert_eq!(
        app.filtered_tasks().len(),
        1,
        "「目标」层级只展示该目标任务"
    );
    app.select_category(c3);
    assert_eq!(
        app.filtered_tasks().len(),
        1,
        "「分类」层级只展示该分类任务"
    );
    app.select_all();
    assert_eq!(app.selected_goal, None, "「全部」应清除目标选择");
    assert_eq!(app.selected_category, None, "「全部」应清除分类选择");
    assert_eq!(app.filtered_tasks().len(), 3, "「全部」应恢复所有任务");
}

/// 删除选中目标并重载后，选择应回落到「全部」而非指向已删除对象。
#[test]
fn reload_drops_selection_of_deleted_goal() {
    let store = open_temp_store("stale");
    let g1 = store.create_goal("G1", None).expect("目标1");
    let c1 = store.create_category(g1, "C1").expect("分类1");
    let g2 = store.create_goal("G2", None).expect("目标2");
    let c2 = store.create_category(g2, "C2").expect("分类2");
    let draft = TaskDraft {
        name: "t".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(c1, draft.clone()).expect("任务1");
    store.create_task(c2, draft).expect("任务2");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    app.select_category(c1);
    let store = app.store.clone();
    store.delete_goal(g1).expect("删除目标1");
    app.reload().expect("重载应成功");

    assert_eq!(app.selected_goal, None, "失效目标应回落全部");
    assert_eq!(app.selected_category, None, "失效分类应清空");
    assert_eq!(app.filtered_tasks().len(), 1, "应展示剩余任务");
}

/// 切换/载入今日页应生成当天「最小行动」列表（默认每日上限 5）。
#[test]
fn today_tab_generates_actions_for_active_tasks() {
    let store = open_temp_store("today-gen");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert_eq!(app.today_actions.len(), 1, "应生成 1 条当日行动");
    assert_eq!(
        app.today_actions[0].status,
        ActionStatus::Pending,
        "新生成行动应为待完成"
    );
}

/// 完成一条今日行动应记为 Done，并可撤销回 Pending。
#[test]
fn today_settle_and_undo() {
    let store = open_temp_store("today-settle");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    let action_id = app.today_actions[0].id;
    app.settle_today_action(action_id, ActionStatus::Done)
        .expect("结算应成功");
    assert_eq!(
        app.today_actions[0].status,
        ActionStatus::Done,
        "应记为完成"
    );

    app.undo_today_action(action_id).expect("撤销应成功");
    assert_eq!(
        app.today_actions[0].status,
        ActionStatus::Pending,
        "撤销后应回到待完成"
    );
}

/// 跳过一条今日行动应记为 Skipped，且可撤销回 Pending。
#[test]
fn today_skip_and_undo() {
    let store = open_temp_store("today-skip");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    let action_id = app.today_actions[0].id;
    app.settle_today_action(action_id, ActionStatus::Skipped)
        .expect("跳过应成功");
    assert_eq!(
        app.today_actions[0].status,
        ActionStatus::Skipped,
        "应记为跳过"
    );
    app.undo_today_action(action_id).expect("撤销应成功");
    assert_eq!(
        app.today_actions[0].status,
        ActionStatus::Pending,
        "撤销后应回到待完成"
    );
}

/// 初始化应载入历史待办首页（过去日期的行动），且不含今日行动。
#[test]
fn history_first_page_loads_past_actions() {
    let store = open_temp_store("hist-1");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");

    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    store.generate_day(past, 5).expect("生成过去一天的行动");

    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    assert_eq!(app.history_actions.len(), 1, "应载入 1 条历史行动");
    assert_eq!(app.history_actions[0].on_date, past, "行动日期应为过去日期");
    assert!(!app.history_has_more, "单条不应有更多页");
}

/// 结算一条历史行动应记为 Done，并写回数据层。
#[test]
fn history_settle_updates_status() {
    let store = open_temp_store("hist-settle");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    store.generate_day(past, 5).expect("生成过去一天的行动");

    let mut app = MinActionApp::new(store.clone(), Arc::new(SystemClock)).expect("构造根视图");

    let action_id = app.history_actions[0].id;
    app.settle_history_action(action_id, ActionStatus::Done)
        .expect("结算应成功");
    assert_eq!(
        app.history_actions[0].status,
        ActionStatus::Done,
        "内存状态应记为完成"
    );

    let stored = store.actions_on(past).expect("读回过去行动");
    assert!(
        stored
            .iter()
            .any(|a| a.id == action_id && a.status == ActionStatus::Done),
        "数据层应记为完成"
    );
}

/// 撤销一条历史行动最近一次结算，应回到待完成。
#[test]
fn history_undo_reverts() {
    let store = open_temp_store("hist-undo");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    store.generate_day(past, 5).expect("生成过去一天的行动");

    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    let action_id = app.history_actions[0].id;
    app.settle_history_action(action_id, ActionStatus::Skipped)
        .expect("跳过应成功");
    app.undo_history_action(action_id).expect("撤销应成功");
    assert_eq!(
        app.history_actions[0].status,
        ActionStatus::Pending,
        "撤销后应回到待完成"
    );
}

/// 已无更多页时，加载更多应是空操作（不重复追加、不报错）。
#[test]
fn history_load_more_noop_when_exhausted() {
    let store = open_temp_store("hist-more");
    let goal = store.create_goal("AI", None).expect("目标");
    let cat = store.create_category(goal, "口语").expect("分类");
    let draft = TaskDraft {
        name: "朗读课文".to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
        description: None,
    };
    store.create_task(cat, draft).expect("任务");
    let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    store.generate_day(past, 5).expect("生成过去一天的行动");

    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");

    let before = app.history_actions.len();
    app.load_history_more().expect("加载更多应成功");
    assert_eq!(app.history_actions.len(), before, "无更多页时不应追加");
}

/// 默认主题偏好应为「跟随系统」。
#[test]
fn default_theme_is_system() {
    let store = open_temp_store("theme-default");
    let app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");
    assert_eq!(app.theme_choice, ThemeChoice::System, "无记录时应跟随系统");
}

/// 设置主题偏好应持久化并可回读。
#[test]
fn set_theme_persists() {
    let store = open_temp_store("theme-set");
    let mut app = MinActionApp::new(store.clone(), Arc::new(SystemClock)).expect("构造根视图");

    app.set_theme(ThemeChoice::Dark).expect("设置主题应成功");
    assert_eq!(app.theme_choice, ThemeChoice::Dark, "内存状态应更新");
    assert_eq!(
        store.theme_choice().expect("读回主题"),
        ThemeChoice::Dark,
        "数据层应持久化"
    );

    app.set_theme(ThemeChoice::Light).expect("设置主题应成功");
    assert_eq!(store.theme_choice().expect("读回主题"), ThemeChoice::Light);
}

/// 设置每日上限应持久化并可回读。
#[test]
fn set_daily_cap_persists() {
    let store = open_temp_store("cap-set");
    let mut app = MinActionApp::new(store.clone(), Arc::new(SystemClock)).expect("构造根视图");

    app.set_daily_cap(3).expect("设置上限应成功");
    assert_eq!(app.daily_cap, 3, "内存状态应更新");
    assert_eq!(store.daily_cap().expect("读回上限"), 3, "数据层应持久化");
}

/// 越界的每日上限应被拒绝，且状态保持不变。
#[test]
fn set_daily_cap_rejects_out_of_range() {
    let store = open_temp_store("cap-invalid");
    let mut app = MinActionApp::new(store, Arc::new(SystemClock)).expect("构造根视图");
    let before = app.daily_cap;

    assert!(app.set_daily_cap(0).is_err(), "0 应越界被拒");
    assert!(app.set_daily_cap(99).is_err(), "99 应越界被拒");
    assert_eq!(app.daily_cap, before, "失败后状态不应改变");
}
