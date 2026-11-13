use gpui::{App, ClickEvent, Window};
use gpui_kit::*;

use super::*;

impl MinActionApp {
    /// 渲染左侧结构树面板。
    pub(crate) fn render_structure(&self, entity: Entity<Self>) -> impl IntoElement {
        let selected_goal = self.selected_goal;
        let selected_category = self.selected_category;
        div()
            .id("structure")
            .w(px(240.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .overflow_y_scroll()
            .child(div().px_3().py_2().child("结构"))
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
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .when(goal_selected, |this| this.bg(rgb(0x2a_2a_2e)))
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
                        if goal_selected {
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
                                        .px_3()
                                        .pl_6()
                                        .py_1()
                                        .rounded_md()
                                        .when(cat_selected, |this| this.bg(rgb(0x2a_2a_2e)))
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
                                            div().flex_1().cursor_pointer().child(format!(
                                                "{}（{}）",
                                                category.name, cat_count
                                            )),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .gap_1()
                                                .child(struct_action(
                                                    "cat-rename",
                                                    "重命名",
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
                        }
                        rows.into_iter()
                    }),
            )
    }
}

/// 结构树每行右侧的小操作按钮（重命名/删除）。
fn struct_action(
    id: &'static str,
    label: &'static str,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_1()
        .py_0()
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(rgb(0x99_99_99))
        .hover(|this| this.text_color(rgb(0xcc_cc_cc)))
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
