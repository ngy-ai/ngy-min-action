use gpui::{App, ClickEvent, Window};
use gpui_kit::*;

use super::style::{SurfaceExt, TEXT_MD, TEXT_XS};
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
            .bg(palette.surface)
            .overflow_y_scroll()
            .child(
                div()
                    .px_3()
                    .py_3()
                    .border_b_1()
                    .child(div().section_title(&palette).child("结构")),
            )
            .child({
                let all_entity = entity.clone();
                let all_selected = selected_goal.is_none();
                div()
                    .id("struct-all")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(all_selected, |this| {
                        this.bg(palette.selected)
                            .border_l_2()
                            .border_color(palette.accent)
                    })
                    .cursor_pointer()
                    .child(div().text_size(TEXT_MD).child("全部"))
                    .child(
                        div()
                            .badge(&palette)
                            .child(self.all_tasks.len().to_string()),
                    )
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
                                .pl_4()
                                .pr_2()
                                .py_2()
                                .rounded_md()
                                .when(goal_selected, |this| {
                                    this.bg(palette.selected)
                                        .border_l_2()
                                        .border_color(palette.accent)
                                })
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
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .text_size(TEXT_MD)
                                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                                .child(goal.name.clone()),
                                        )
                                        .child(div().badge(&palette).child(count.to_string())),
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
                                    .pl_7()
                                    .pr_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .when(cat_selected, |this| {
                                        this.bg(palette.selected)
                                            .border_l_2()
                                            .border_color(palette.accent)
                                    })
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
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .cursor_pointer()
                                            .child(
                                                div()
                                                    .text_size(TEXT_XS)
                                                    .child(category.name.clone()),
                                            )
                                            .child(
                                                div().badge(&palette).child(cat_count.to_string()),
                                            ),
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

/// 结构树每行右侧的小操作按钮（重命名/删除）。
fn struct_action(
    id: &'static str,
    label: &'static str,
    palette: &Palette,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .soft_button(palette, false)
        .px_2()
        .text_size(TEXT_XS)
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
