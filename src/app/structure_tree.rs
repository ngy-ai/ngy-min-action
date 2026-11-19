use gpui::{App, ClickEvent, Window};
use gpui_kit::*;

use super::*;
use crate::palette::Palette;

impl MinActionApp {
    /// 渲染左侧结构树面板。
    pub(crate) fn render_structure(&self, entity: Entity<Self>) -> impl IntoElement {
        let selected_goal = self.selected_goal;
        let selected_category = self.selected_category;
        let palette = self.palette;
        div()
            .id("structure")
            .w(px(240.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .overflow_y_scroll()
            .child(div().px_3().py_2().child("结构"))
            .child({
                let goal_root = entity.clone();
                let category_root = entity.clone();
                let task_root = entity.clone();
                let goal_scope = selected_goal.or_else(|| self.all_goals.first().map(|g| g.id));
                let category_scope =
                    selected_category.or_else(|| self.all_categories.first().map(|c| c.id));
                let goal_label = goal_scope
                    .and_then(|id| self.all_goals.iter().find(|goal| goal.id == id))
                    .map(|goal| format!("归属：{}", goal.name))
                    .unwrap_or_else(|| "请先创建目标".to_owned());
                let category_label = category_scope
                    .and_then(|id| self.all_categories.iter().find(|c| c.id == id))
                    .map(|category| format!("归属：{}", category.name))
                    .unwrap_or_else(|| "请先创建目标与分类".to_owned());

                div()
                    .id("create-actions")
                    .px_3()
                    .pb_2()
                    .flex()
                    .gap_2()
                    .child(create_action(
                        "add-goal",
                        "+ 目标",
                        &palette,
                        move |window, cx| {
                            goal_root.update(cx, |root, cx| {
                                let app_entity = cx.entity();
                                root.open_create(
                                    create_dialog::CreateKind::Goal,
                                    "新建目标".to_owned(),
                                    app_entity,
                                    window,
                                    cx,
                                );
                            });
                        },
                    ))
                    .child(create_action(
                        "add-category",
                        "+ 分类",
                        &palette,
                        move |window, cx| {
                            category_root.update(cx, |root, cx| {
                                let app_entity = cx.entity();
                                root.open_create(
                                    create_dialog::CreateKind::Category { goal: goal_scope },
                                    goal_label.clone(),
                                    app_entity,
                                    window,
                                    cx,
                                );
                            });
                        },
                    ))
                    .child(create_action(
                        "add-task",
                        "+ 任务",
                        &palette,
                        move |window, cx| {
                            task_root.update(cx, |root, cx| {
                                let app_entity = cx.entity();
                                root.open_create(
                                    create_dialog::CreateKind::Task {
                                        category: category_scope,
                                    },
                                    category_label.clone(),
                                    app_entity,
                                    window,
                                    cx,
                                );
                            });
                        },
                    ))
            })
            .child({
                let all_entity = entity.clone();
                let all_selected = selected_goal.is_none();
                div()
                    .id("struct-all")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .when(all_selected, |this| this.bg(self.palette.selected))
                    .cursor_pointer()
                    .child(format!("全部（{}）", self.all_tasks.len()))
                    .on_click(move |_ev, _window, cx| {
                        all_entity.update(cx, |this, cx| {
                            this.select_all();
                            cx.notify();
                        });
                    })
            })
            .children(
                self.all_goals
                    .iter()
                    .enumerate()
                    .flat_map(move |(gi, goal)| {
                        let goal_id = goal.id;
                        let goal_selected = selected_goal == Some(goal_id);
                        let count = self.goal_task_count(goal_id);
                        let entity_goal = entity.clone();
                        let entity_cat = entity.clone();
                        let rename_entity = entity.clone();
                        let delete_entity = entity.clone();
                        let goal_name = goal.name.clone();
                        let mut rows = vec![{
                            div()
                                .id(format!("goal-{gi}"))
                                .pl_5()
                                .pr_3()
                                .py_1()
                                .rounded_md()
                                .when(goal_selected, |this| this.bg(self.palette.selected))
                                .flex()
                                .items_center()
                                .justify_between()
                                .on_click(move |_ev, _window, cx| {
                                    entity_goal.update(cx, |this, cx| {
                                        this.select_goal(goal_id);
                                        cx.notify();
                                    });
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .cursor_pointer()
                                        .child(format!("{}（{}）", goal.name, count)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_1()
                                        .child(struct_action(
                                            "goal-rename",
                                            "重命名",
                                            &palette,
                                            move |_ev, window, cx| {
                                                open_rename(
                                                    rename_entity.clone(),
                                                    StructTarget::Goal(goal_id),
                                                    &goal_name,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                        .child(struct_action(
                                            "goal-delete",
                                            "删除",
                                            &palette,
                                            move |_ev, window, cx| {
                                                let impact = delete_entity
                                                    .read(cx)
                                                    .goal_deletion_impact(goal_id);
                                                open_delete(
                                                    delete_entity.clone(),
                                                    StructTarget::Goal(goal_id),
                                                    impact,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        )),
                                )
                        }];
                        for (ci, category) in self
                            .all_categories
                            .iter()
                            .filter(|category| category.goal_id == goal_id)
                            .enumerate()
                        {
                            let category_id = category.id;
                            let cat_selected = selected_category == Some(category_id);
                            let cat_count =
                                self.task_counts.get(&category_id).copied().unwrap_or(0);
                            let cat_select = entity_cat.clone();
                            let cat_rename = entity_cat.clone();
                            let cat_delete = entity_cat.clone();
                            let cat_name = category.name.clone();
                            rows.push(
                                div()
                                    .id(format!("cat-{gi}-{ci}"))
                                    .pl_8()
                                    .pr_3()
                                    .py_1()
                                    .rounded_md()
                                    .when(cat_selected, |this| this.bg(self.palette.selected))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .on_click(move |_ev, _window, cx| {
                                        cat_select.update(cx, |this, cx| {
                                            this.select_category(category_id);
                                            cx.notify();
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .cursor_pointer()
                                            .child(format!("{}（{}）", category.name, cat_count)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(struct_action(
                                                "cat-rename",
                                                "重命名",
                                                &palette,
                                                move |_ev, window, cx| {
                                                    open_rename(
                                                        cat_rename.clone(),
                                                        StructTarget::Category(category_id),
                                                        &cat_name,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(struct_action(
                                                "cat-delete",
                                                "删除",
                                                &palette,
                                                move |_ev, window, cx| {
                                                    let impact = cat_delete
                                                        .read(cx)
                                                        .category_deletion_impact(category_id);
                                                    open_delete(
                                                        cat_delete.clone(),
                                                        StructTarget::Category(category_id),
                                                        impact,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            )),
                                    ),
                            );
                        }
                        rows.into_iter()
                    }),
            )
    }
}

/// 结构树顶部的「新建」按钮。
fn create_action(
    id: &'static str,
    label: &'static str,
    palette: &Palette,
    handler: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_md()
        .bg(palette.inactive_pill)
        .text_size(px(11.))
        .cursor_pointer()
        .child(label)
        .on_click(move |_ev, window, cx| handler(window, cx))
}

/// 结构树每行右侧的小操作按钮（重命名/删除）。
fn struct_action(
    id: &'static str,
    label: &'static str,
    palette: &Palette,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(palette.text_muted)
        .hover(|this| this.text_color(palette.text))
        .child(label)
        .on_click(handler)
}

/// 打开重命名对话框：构造对话框实体并挂到根视图。
fn open_rename(
    root: Entity<MinActionApp>,
    target: StructTarget,
    current_name: &str,
    window: &mut Window,
    cx: &mut App,
) {
    let store = root.read(cx).store.clone();
    let dialog = cx.new(|c| {
        rename_dialog::RenameDialog::new(target, current_name, store, root.clone(), window, c)
    });
    root.update(cx, |this, cx| {
        this.dialog = Some(StructDialog::Rename(dialog));
        cx.notify();
    });
}

/// 打开删除确认对话框：先算影响范围，再构造对话框实体并挂到根视图。
fn open_delete(
    root: Entity<MinActionApp>,
    target: StructTarget,
    impact: DeletionImpact,
    window: &mut Window,
    cx: &mut App,
) {
    let store = root.read(cx).store.clone();
    let dialog = cx
        .new(|c| delete_dialog::DeleteDialog::new(target, impact, store, root.clone(), window, c));
    root.update(cx, |this, cx| {
        this.dialog = Some(StructDialog::Delete(dialog));
        cx.notify();
    });
}
