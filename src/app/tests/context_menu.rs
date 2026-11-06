//! 011：结构树右键菜单的 headless 验收。

use eframe::egui;

use super::*;
use crate::app::structure_menu;
use crate::app::theme;
use crate::model::ThemeChoice;

#[test]
fn menu_items_are_exactly_rename_and_delete() {
    let items = structure_menu::MENU_ITEMS;
    assert_eq!(items.len(), 2);
    let labels: Vec<_> = items.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, ["改名", "删除"]);
    let actions: Vec<_> = items.iter().map(|(_, action)| *action).collect();
    assert!(actions.contains(&structure_menu::StructureAction::Rename));
    assert!(actions.contains(&structure_menu::StructureAction::Delete));
}

#[test]
fn menu_renders_in_both_themes() {
    let ctx = egui::Context::default();
    theme::apply_styles(&ctx);
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(theme::preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(structure_menu::show(ui).is_none());
        });
        output.textures_delta.clear();
    }
}

#[test]
fn structure_tree_with_context_menus_renders_in_both_themes() {
    let mut app = open_app("context-menu-tree");
    let category = seed_category(&mut app, "学习英语", "口语");
    app.store
        .create_task(category, draft("连读"))
        .expect("创建任务");
    app.reload().expect("刷新界面数据");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(theme::preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.ui_main_view(ui);
        });
        output.textures_delta.clear();
    }
}

#[test]
fn name_context_menu_opens_on_secondary_click() {
    use std::cell::Cell;

    let ctx = egui::Context::default();
    theme::apply_styles(&ctx);
    let rect = Cell::new(None);
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let name = ui.selectable_label(false, "分类1");
        rect.set(Some(name.rect));
        name.context_menu(|menu| {
            structure_menu::show(menu);
        });
    });
    output.textures_delta.clear();

    let center = rect.get().expect("名称控件必须先完成一帧布局").center();
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(center),
                egui::Event::PointerButton {
                    pos: center,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
                egui::Event::PointerButton {
                    pos: center,
                    button: egui::PointerButton::Secondary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        },
        |ui| {
            ui.selectable_label(false, "分类1").context_menu(|menu| {
                structure_menu::show(menu);
            });
        },
    );
    output.textures_delta.clear();

    let opened = Cell::new(false);
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.selectable_label(false, "分类1").context_menu(|_| {
            opened.set(true);
        });
    });
    output.textures_delta.clear();
    assert!(opened.get(), "右键名称后菜单必须保持打开");
}

#[test]
fn delete_item_uses_the_semantic_error_color() {
    let ctx = egui::Context::default();
    theme::apply_styles(&ctx);
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        ctx.set_theme(theme::preference(choice));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_ne!(
                ui.visuals().error_fg_color,
                ui.visuals().text_color(),
                "危险色必须与普通文字色不同"
            );
            structure_menu::show(ui);
        });
        output.textures_delta.clear();
    }
}
