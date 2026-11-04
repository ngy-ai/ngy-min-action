//! FR-002 / FR-004 / FR-005 / FR-009 / FR-011 / FR-017 / FR-034 存储行为。

use chrono::NaiveDate;
use ngy_min_action::model::{
    ActionStatus, DomainError, Frequency, TaskDraft, TaskStatus, validate_name,
};
use ngy_min_action::store::Store;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试用的日期必须合法")
}

fn open_store(tag: &str) -> Store {
    let dir =
        std::env::temp_dir().join(format!("ngy-min-action-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(&dir.join("data.db")).expect("应能创建临时数据库")
}

fn draft(name: &str) -> TaskDraft {
    TaskDraft {
        name: name.to_owned(),
        frequency: Frequency::Daily,
        estimated_minutes: None,
    }
}

#[test]
fn duplicate_category_in_the_same_goal_is_rejected() {
    let store = open_store("dup-category");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.create_category(goal, "口语").expect("首次创建分类");
    let error = store
        .create_category(goal, "口语")
        .expect_err("同目标下重名必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
    assert_eq!(store.categories(goal).expect("列出分类").len(), 1);
}

#[test]
fn the_same_category_name_is_allowed_across_goals() {
    let store = open_store("cross-goal");
    let first = store.create_goal("学习英语", None).expect("创建目标");
    let second = store.create_goal("健身", None).expect("创建目标");
    store.create_category(first, "口语").expect("首次创建分类");
    store
        .create_category(second, "口语")
        .expect("跨目标允许重名");
}

#[test]
fn blank_and_control_character_names_are_rejected() {
    assert!(validate_name("   ", "目标名称", 80).is_err());
    assert!(validate_name("学习\n英语", "目标名称", 80).is_err());
    assert!(validate_name(&"长".repeat(81), "目标名称", 80).is_err());
    assert!(validate_name("  学习英语  ", "目标名称", 80).is_ok());
}

#[test]
fn soft_deleted_category_disappears_from_lists() {
    let store = open_store("soft-delete");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let impact = store.delete_category(category).expect("删除分类");
    assert_eq!(impact.categories, 0);
    assert!(store.categories(goal).expect("列出分类").is_empty());
    let error = store
        .create_category(goal, "口语")
        .expect_err("软删除不释放名称，重建同名分类必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
}

#[test]
fn deleting_a_goal_reports_its_impact() {
    let store = open_store("goal-impact");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    let impact = store.delete_goal(goal).expect("删除目标");
    assert_eq!(impact.categories, 1);
    assert_eq!(impact.tasks, 1);
    assert_eq!(impact.pending_actions, 1);
    assert!(store.goals().expect("列出目标").is_empty());
}

#[test]
fn completed_actions_survive_goal_deletion() {
    let store = open_store("history-kept");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");

    store.delete_goal(goal).expect("删除目标");
    let remaining = store.actions_on(date(2026, 11, 2)).expect("查询当天行动");
    assert_eq!(remaining.len(), 1, "已完成的历史记录必须保留");
    assert_eq!(remaining[0].status, ActionStatus::Done);
}

#[test]
fn generate_day_is_idempotent() {
    let store = open_store("idempotent");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("背 10 个单词"))
        .expect("创建任务");

    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    let second = store.generate_day(date(2026, 11, 2), 5).expect("重复生成");
    assert_eq!(first.len(), 1);
    assert_eq!(
        first.iter().map(|item| item.id).collect::<Vec<_>>(),
        second.iter().map(|item| item.id).collect::<Vec<_>>()
    );
}

#[test]
fn daily_cap_limits_generated_actions() {
    let store = open_store("cap");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "综合").expect("创建分类");
    for index in 1..=8 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    let pending = actions
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();
    assert_eq!(pending, 5, "全局上限必须生效");
    assert_eq!(
        store.deferred_count(date(2026, 11, 2)).expect("顺延计数"),
        3
    );
}

#[test]
fn adding_a_task_does_not_exceed_the_cap() {
    let store = open_store("cap-late-task");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    for index in 1..=5 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }
    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    assert_eq!(first.len(), 5);

    store
        .create_task(category, draft("任务6"))
        .expect("创建任务");
    let after = store.generate_day(date(2026, 11, 2), 5).expect("再次生成");
    let pending = after
        .iter()
        .filter(|item| item.status == ActionStatus::Pending)
        .count();
    assert_eq!(pending, 5, "当天已占满上限时，新增任务不得突破上限");
    assert_eq!(
        store.deferred_count(date(2026, 11, 2)).expect("顺延计数"),
        1,
        "被挡下的新任务应计入顺延"
    );
}

#[test]
fn new_actions_append_after_existing_seq() {
    let store = open_store("seq-append");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    for index in 1..=2 {
        store
            .create_task(category, draft(&format!("任务{index}")))
            .expect("创建任务");
    }
    let first = store.generate_day(date(2026, 11, 2), 5).expect("首次生成");
    let max_seq = first.iter().map(|item| item.seq).max().expect("应有行动");

    store
        .create_task(category, draft("任务3"))
        .expect("创建任务");
    let after = store.generate_day(date(2026, 11, 2), 5).expect("再次生成");
    let appended = after.last().expect("应有行动");
    assert!(
        appended.seq > max_seq,
        "新行动必须追加到清单末尾，实际 seq={}，已有最大 seq={max_seq}",
        appended.seq
    );
}

#[test]
fn paused_task_produces_no_action() {
    let store = open_store("paused");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    assert!(actions.is_empty(), "暂停的任务不产生行动");
}

#[test]
fn completing_an_action_advances_review_progress() {
    let store = open_store("review-advance");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");
    let progress = store.review_progress(task).expect("读取复习进度");
    assert_eq!(progress.level, 1);
    assert!(progress.next_review_on.is_some());
}

#[test]
fn undo_restores_the_previous_state() {
    let store = open_store("undo");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("标记完成");
    store.undo_last_settlement(actions[0].id).expect("撤销结算");

    let progress = store.review_progress(task).expect("读取复习进度");
    assert_eq!(progress.level, 0, "撤销后复习进度必须回退");
    let after = store.actions_on(date(2026, 11, 2)).expect("查询当天行动");
    assert_eq!(after[0].status, ActionStatus::Pending);
}

#[test]
fn deletion_impact_is_available_before_deleting() {
    let store = open_store("impact");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("每周学习一个连读"))
        .expect("创建任务");
    store.generate_day(date(2026, 11, 2), 5).expect("生成行动");

    let impact = store
        .goal_deletion_impact(goal)
        .expect("删除前应能预览目标的影响范围");
    assert_eq!(impact.categories, 1);
    assert_eq!(impact.tasks, 1);
    assert_eq!(impact.pending_actions, 1);
    assert_eq!(
        store.goals().expect("目标仍在").len(),
        1,
        "预览不得修改数据"
    );
    assert_eq!(
        store.tasks(category).expect("任务仍在").len(),
        1,
        "预览不得修改数据"
    );

    let impact = store
        .category_deletion_impact(category)
        .expect("删除前应能预览分类的影响范围");
    assert_eq!(impact.tasks, 1);
    assert_eq!(impact.pending_actions, 1);
}

#[test]
fn only_settled_actions_offer_undo() {
    assert!(ActionStatus::Done.is_settled());
    assert!(ActionStatus::Skipped.is_settled());
    assert!(!ActionStatus::Pending.is_settled(), "待完成没有结算记录");
    assert!(
        !ActionStatus::Deferred.is_settled(),
        "顺延从未结算，界面上不能给撤销入口"
    );
    assert!(
        !ActionStatus::Expired.is_settled(),
        "过期从未结算，界面上不能给撤销入口"
    );
}

#[test]
fn daily_cap_setting_is_validated() {
    let store = open_store("daily-cap");
    assert_eq!(store.daily_cap().expect("默认上限"), 5);
    store.set_daily_cap(3).expect("设置上限");
    assert_eq!(store.daily_cap().expect("读取上限"), 3);
    assert!(store.set_daily_cap(0).is_err(), "0 必须被拒绝");
    assert!(store.set_daily_cap(21).is_err(), "超出上限必须被拒绝");
}

#[test]
fn upgrading_an_old_database_clears_business_data() {
    let dir = std::env::temp_dir().join(format!(
        "ngy-min-action-test-{}-migrate",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("data.db");

    {
        let store = Store::open(&path).expect("创建库");
        let goal = store.create_goal("学习英语", None).expect("创建目标");
        let category = store.create_category(goal, "口语").expect("创建分类");
        store
            .create_task(category, draft("连读"))
            .expect("创建任务");
        store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
        store.set_daily_cap(3).expect("设置每日上限");
    }

    {
        let connection = rusqlite::Connection::open(&path).expect("打开旧库");
        connection
            .pragma_update(None, "user_version", 1_i64)
            .expect("标记为旧版本");
    }

    let store = Store::open(&path).expect("旧库应能升级");
    assert!(
        store.goals().expect("列出目标").is_empty(),
        "升级必须清空业务数据"
    );
    assert_eq!(
        store.daily_cap().expect("读取上限"),
        5,
        "用户设置随升级重置为默认值"
    );
    store.create_goal("学习英语", None).expect("升级后仍可写入");
    assert_eq!(store.goals().expect("列出目标").len(), 1);

    drop(store);
    let reopened = Store::open(&path).expect("再次打开");
    assert_eq!(
        reopened.goals().expect("列出目标").len(),
        1,
        "升级只清空一次"
    );
}

#[test]
fn duplicate_goal_names_are_rejected() {
    let store = open_store("dup-goal");
    store.create_goal("学习英语", None).expect("首次创建目标");
    let error = store
        .create_goal("学习英语", None)
        .expect_err("同名目标必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
    assert_eq!(store.goals().expect("列出目标").len(), 1, "失败不得落行");
}

#[test]
fn goal_names_ignore_case_and_surrounding_spaces() {
    let store = open_store("goal-case");
    store
        .create_goal("learning english", None)
        .expect("首次创建目标");
    assert!(
        store.create_goal("Learning English", None).is_err(),
        "只有大小写不同也算重名"
    );
    assert!(
        store.create_goal("  learning english  ", None).is_err(),
        "两端空白不构成差异"
    );
    assert_eq!(store.goals().expect("列出目标").len(), 1);
}

#[test]
fn full_unicode_case_folding_catches_expansion_cases() {
    let store = open_store("goal-unicode");
    store.create_goal("straße", None).expect("首次创建目标");
    assert!(
        store.create_goal("STRASSE", None).is_err(),
        "ß 折叠为 ss 后必须被判重"
    );
    store.create_goal("ΣΟΦΟΣ", None).expect("希腊字母目标");
    assert!(
        store.create_goal("σοφος", None).is_err(),
        "希腊字母大小写必须折叠"
    );
}

#[test]
fn renaming_a_goal_to_an_existing_name_is_rejected() {
    let store = open_store("goal-rename");
    store.create_goal("学习英语", None).expect("创建目标");
    let other = store.create_goal("日语", None).expect("创建目标");
    let error = store
        .rename_goal(other, "学习英语")
        .expect_err("重命名到已有名称必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
    assert_eq!(
        store
            .goals()
            .expect("列出目标")
            .iter()
            .filter(|goal| goal.name == "日语")
            .count(),
        1,
        "原名称必须保持不变"
    );
}

#[test]
fn a_deleted_goal_name_cannot_be_reused() {
    let store = open_store("goal-reuse");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.delete_goal(goal).expect("删除目标");
    let error = store
        .create_goal("学习英语", None)
        .expect_err("软删除不释放名称");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
}

#[test]
fn duplicate_category_names_are_rejected_within_a_goal() {
    let store = open_store("dup-category-case");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.create_category(goal, "Listening").expect("创建分类");
    assert!(
        store.create_category(goal, "listening").is_err(),
        "仅大小写不同的分类名必须被拒绝"
    );
    assert_eq!(store.categories(goal).expect("列出分类").len(), 1);
}

#[test]
fn renaming_a_category_to_an_existing_name_is_rejected() {
    let store = open_store("category-rename");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store.create_category(goal, "口语").expect("创建分类");
    let grammar = store.create_category(goal, "语法").expect("创建分类");
    let error = store
        .rename_category(grammar, "口语")
        .expect_err("同目标下重命名必须被拒绝");
    assert!(matches!(error, DomainError::DuplicateName { .. }));
    assert_eq!(store.categories(goal).expect("列出分类").len(), 2);
}

#[test]
fn names_are_stored_without_surrounding_spaces() {
    let store = open_store("trim");
    let goal = store.create_goal("  学习英语  ", None).expect("创建目标");
    assert_eq!(store.goals().expect("列出目标")[0].name, "学习英语");
    store.create_category(goal, "  口语  ").expect("创建分类");
    assert_eq!(store.categories(goal).expect("列出分类")[0].name, "口语");
}

#[test]
fn inner_spaces_are_preserved() {
    let store = open_store("inner-space");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    store
        .create_category(goal, "学习 英语")
        .expect("创建带内部空格的分类");
    store
        .create_category(goal, "学习英语")
        .expect("内部空格不同的名称是两条");
    assert_eq!(store.categories(goal).expect("列出分类").len(), 2);
}

#[test]
fn blank_goal_names_are_rejected_without_writing() {
    let store = open_store("blank-goal");
    assert!(store.create_goal("   ", None).is_err(), "纯空白必须被拒绝");
    assert!(store.goals().expect("列出目标").is_empty(), "不得落行");
}

#[test]
fn task_open_actions_counts_unsettled_actions() {
    let store = open_store("task-open-actions");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    assert_eq!(
        store.task_open_actions(task).expect("无行动"),
        0,
        "没有行动时应为 0"
    );

    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    assert_eq!(
        store.task_open_actions(task).expect("一条待完成"),
        1,
        "一条 pending 行动应计为 1"
    );

    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成行动");
    assert_eq!(
        store.task_open_actions(task).expect("结算后"),
        0,
        "已完成行动不应计入"
    );

    store.delete_task(task).expect("删除任务");
    assert_eq!(
        store.task_open_actions(task).expect("删除后"),
        0,
        "已删除任务返回 0"
    );
}

#[test]
fn all_tasks_includes_paused_tasks_for_name_resolution() {
    let store = open_store("all-tasks");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store
        .set_task_status(task, TaskStatus::Paused)
        .expect("暂停任务");
    let names: Vec<String> = store
        .all_tasks()
        .expect("全部任务")
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(
        names,
        vec!["连读".to_string()],
        "暂停任务的名称仍必须可解析，否则今日清单会退化为任务#N"
    );
}

#[test]
fn tasks_are_stored_with_their_frequency() {
    let store = open_store("task-frequency");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(
            category,
            TaskDraft {
                name: "隔三天".to_string(),
                frequency: Frequency::EveryNDays(3),
                estimated_minutes: None,
            },
        )
        .expect("创建任务");
    store
        .create_task(
            category,
            TaskDraft {
                name: "每周两次".to_string(),
                frequency: Frequency::WeeklyN(2),
                estimated_minutes: None,
            },
        )
        .expect("创建任务");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].frequency, Frequency::EveryNDays(3));
    assert_eq!(tasks[1].frequency, Frequency::WeeklyN(2));
}

#[test]
fn progress_for_category_returns_progress_of_live_tasks_only() {
    let store = open_store("progress-batch");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    assert!(
        store
            .progress_for_category(category)
            .expect("空分类")
            .is_empty(),
        "空分类应返回空向量"
    );

    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    let actions = store.generate_day(date(2026, 11, 2), 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成行动");

    let progress = store.progress_for_category(category).expect("读取进度");
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].task_id, task);
    assert_eq!(progress[0].level, 1);

    store.delete_task(task).expect("删除任务");
    assert!(
        store
            .progress_for_category(category)
            .expect("读取进度")
            .is_empty(),
        "已删除任务的进度不应出现"
    );
}

#[test]
fn update_task_frequency_only_touches_the_frequency_columns() {
    let store = open_store("update-frequency");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    store
        .update_task_frequency(task, Frequency::EveryNDays(3))
        .expect("改成每 3 天");

    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].frequency, Frequency::EveryNDays(3));
    assert_eq!(tasks[0].name, "连读", "名称不应变化");
    assert_eq!(tasks[0].status, TaskStatus::Active, "状态不应变化");
}

#[test]
fn update_task_frequency_rejects_deleted_task() {
    let store = open_store("update-frequency-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store.delete_task(task).expect("删除任务");

    let error = store
        .update_task_frequency(task, Frequency::Daily)
        .expect_err("已删除任务必须拒绝");
    assert!(
        matches!(error, DomainError::NotFound { .. }),
        "应返回 NotFound，实际为 {error:?}"
    );
}

#[test]
fn update_task_estimated_minutes_sets_and_clears_the_value() {
    let store = open_store("update-estimate");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    store
        .update_task_estimated_minutes(task, Some(25))
        .expect("设置耗时");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].estimated_minutes, Some(25));
    assert_eq!(tasks[0].name, "连读", "名称不应变化");
    assert_eq!(tasks[0].frequency, Frequency::Daily, "频率不应变化");
    assert_eq!(tasks[0].status, TaskStatus::Active, "状态不应变化");

    store
        .update_task_estimated_minutes(task, None)
        .expect("清空耗时");
    let tasks = store.tasks(category).expect("列出任务");
    assert_eq!(tasks[0].estimated_minutes, None, "清空后应为未估时");
    assert_eq!(tasks[0].name, "连读", "清空耗时不应影响其它字段");
}

#[test]
fn update_task_estimated_minutes_rejects_deleted_task() {
    let store = open_store("update-estimate-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    store.delete_task(task).expect("删除任务");

    let error = store
        .update_task_estimated_minutes(task, Some(10))
        .expect_err("已删除任务必须拒绝");
    assert!(
        matches!(error, DomainError::NotFound { .. }),
        "应返回 NotFound，实际为 {error:?}"
    );
}

fn generate(store: &Store, day: NaiveDate) {
    let actions = store.generate_day(day, 5).expect("生成当天行动");
    assert!(!actions.is_empty(), "当天应当生成行动");
}

fn settle_day(store: &Store, day: NaiveDate, status: ActionStatus) {
    let actions = store.generate_day(day, 5).expect("生成当天行动");
    assert!(!actions.is_empty(), "当天应当生成行动");
    for action in &actions {
        store.settle_action(action.id, status).expect("结算行动");
    }
}

#[test]
fn review_stats_counts_settled_actions_by_status() {
    let store = open_store("stats-tally");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 2), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.window_days, 30);
    assert_eq!(stats.tally.done, 3);
    assert_eq!(stats.tally.skipped, 0);
    assert_eq!(stats.tally.expired, 0);
    assert_eq!(stats.tally.completion_percent(), Some(100));
}

#[test]
fn review_stats_reports_empty_tally_when_nothing_is_settled() {
    let store = open_store("stats-empty");
    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.tally.settled(), 0);
    assert_eq!(stats.tally.completion_percent(), None, "没有可统计的行动");
    assert_eq!(stats.current_streak, 0);
    assert_eq!(stats.longest_streak, 0);
}

#[test]
fn review_stats_breaks_the_streak_on_a_skipped_day() {
    let store = open_store("stats-streak-break");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 10, 30), ActionStatus::Done);
    settle_day(&store, date(2026, 10, 31), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 2), ActionStatus::Skipped);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 1, "从 11-03 起算，11-02 中断");
    assert_eq!(stats.longest_streak, 3, "10-30 到 11-01 是历史最长");
}

#[test]
fn review_stats_skips_days_without_actions() {
    let store = open_store("stats-streak-gap");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 1), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);

    let stats = store.review_stats(date(2026, 11, 3), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 2, "没有行动的日子不算中断");
}

#[test]
fn review_stats_keeps_the_streak_when_today_is_unfinished() {
    let store = open_store("stats-streak-today");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    settle_day(&store, date(2026, 11, 2), ActionStatus::Done);
    settle_day(&store, date(2026, 11, 3), ActionStatus::Done);
    generate(&store, date(2026, 11, 4));

    let stats = store.review_stats(date(2026, 11, 4), 30).expect("读取统计");
    assert_eq!(stats.current_streak, 2, "今天还没完成，不应打断");
    assert_eq!(stats.tally.settled(), 2, "今天的 pending 不计入已结算");
}

#[test]
fn review_stats_groups_tasks_by_stage() {
    let store = open_store("stats-stages");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    store.create_task(category, draft("连读")).expect("任务一");
    store.create_task(category, draft("音标")).expect("任务二");
    store.create_task(category, draft("语调")).expect("任务三");

    let today = date(2026, 11, 4);
    let actions = store.generate_day(today, 5).expect("生成行动");
    store
        .settle_action(actions[0].id, ActionStatus::Done)
        .expect("完成一条，把对应任务推到第 1 级");

    let stats = store.review_stats(today, 30).expect("读取统计");
    assert_eq!(stats.untouched_tasks, 2);
    assert_eq!(stats.learning_tasks, 1);
    assert_eq!(stats.graduated_tasks, 0);
}

#[test]
fn review_stats_keeps_expired_actions_but_drops_deleted_tasks_from_stages() {
    let store = open_store("stats-deleted");
    let goal = store.create_goal("学习英语", None).expect("创建目标");
    let category = store.create_category(goal, "口语").expect("创建分类");
    let task = store
        .create_task(category, draft("连读"))
        .expect("创建任务");

    let today = date(2026, 11, 4);
    generate(&store, today);
    store.delete_task(task).expect("删除任务");

    let stats = store.review_stats(today, 30).expect("读取统计");
    assert_eq!(stats.tally.expired, 1, "删除后未完成行动转为过期");
    assert_eq!(stats.tally.completion_percent(), Some(0));
    assert_eq!(
        stats.untouched_tasks + stats.learning_tasks + stats.graduated_tasks,
        0,
        "阶段分布不应包含已删除的任务"
    );
}
