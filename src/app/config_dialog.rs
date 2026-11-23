//! 统一配置窗口：左侧 Tab 切换「每日上限 / 存储地址 / 主题」。
//!
//! 存储地址改动复用 `storage::relocate` 做在线备份、完整性校验与位置配置写入；
//! 成功后替换根视图连接并清理旧数据，失败则保留旧位置并就地显示原因。
//! 主题改动即写设置并应用到当前窗口。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{App, Context, Entity, Render, Window};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::MinActionApp;
use crate::app::storage_dialog;
use crate::app::{apply_theme_choice, theme_label};
use crate::common::DAILY_CAP_DEFAULT;
use crate::config::Bootstrap;
use crate::model::ThemeChoice;
use crate::palette::Palette;
use crate::storage;
use crate::store::Store;

use super::style::{SurfaceExt, TEXT_MD, Tone, dialog_title, field_label, info_row};

/// 配置窗口左侧页签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigTab {
    /// 每日行动上限。
    DailyCap,
    /// 数据目录位置。
    Storage,
    /// 界面主题偏好。
    Theme,
}

impl ConfigTab {
    /// 左侧页签的展示顺序。
    const ALL: [ConfigTab; 3] = [ConfigTab::DailyCap, ConfigTab::Storage, ConfigTab::Theme];

    /// 页签文案。
    fn label(self) -> &'static str {
        match self {
            ConfigTab::DailyCap => "每日上限",
            ConfigTab::Storage => "存储地址",
            ConfigTab::Theme => "主题",
        }
    }
}

/// 统一配置窗口视图。
pub struct ConfigDialog {
    tab: ConfigTab,
    store: Arc<Store>,
    bootstrap: Bootstrap,
    root: Entity<MinActionApp>,
    cap: u8,
    /// 当前主题偏好，供「主题」页高亮已选项。
    theme: ThemeChoice,
    path_input: Entity<InputState>,
    error: Option<String>,
    notice: Option<String>,
}

impl ConfigDialog {
    /// 构造配置窗口并定位到指定页签。
    pub(crate) fn new(
        tab: ConfigTab,
        store: Arc<Store>,
        bootstrap: Bootstrap,
        root: Entity<MinActionApp>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let cap = store.daily_cap().unwrap_or(DAILY_CAP_DEFAULT);
        let theme = store.theme_choice().unwrap_or_default();
        let current = store.data_dir().to_path_buf();
        let path_input = cx
            .new(|c| InputState::new(window, c).default_value(current.to_string_lossy().as_ref()));
        Self {
            tab,
            store,
            bootstrap,
            root,
            cap,
            theme,
            path_input,
            error: None,
            notice: None,
        }
    }
}

impl Render for ConfigDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::from_cx(cx);
        let dialog = cx.entity().clone();
        let root = self.root.clone();
        let tab = self.tab;
        let cap = self.cap;
        let theme = self.theme;
        let path_input = self.path_input.clone();
        let store = self.store.clone();
        let error = self.error.clone();
        let notice = self.notice.clone();

        div()
            .id("config-dialog")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(640.))
            .child(dialog_title("配置"))
            .child(
                div()
                    .id("config-body")
                    .flex()
                    .min_h(px(260.))
                    .child(tab_list(dialog.clone(), tab, &p))
                    .child(
                        div()
                            .id("config-content")
                            .flex()
                            .flex_col()
                            .gap_2()
                            .pl_4()
                            .flex_1()
                            .when(tab == ConfigTab::DailyCap, |this| {
                                this.child(daily_cap_content(dialog.clone(), root.clone(), cap, &p))
                            })
                            .when(tab == ConfigTab::Storage, |this| {
                                this.child(storage_content(
                                    dialog.clone(),
                                    path_input.clone(),
                                    store.clone(),
                                    &p,
                                ))
                            })
                            .when(tab == ConfigTab::Theme, |this| {
                                this.child(theme_content(dialog.clone(), root.clone(), theme, &p))
                            }),
                    ),
            )
            .when_some(error, |this, msg| {
                this.child(div().text_color(p.danger_fg).child(msg))
            })
            .when_some(notice, |this, msg| {
                this.child(div().text_color(p.text_muted).child(msg))
            })
            .child(
                div().flex().gap_2().justify_end().child(
                    div()
                        .id("config-close")
                        .tone_button(&p, Tone::Neutral)
                        .px_3()
                        .child("关闭")
                        .on_click(move |_ev, _window, cx| {
                            root.update(cx, |root, cx| {
                                root.close_dialog();
                                cx.notify();
                            });
                        }),
                ),
            )
    }
}

/// 左侧 Tab 列表。
fn tab_list(dialog: Entity<ConfigDialog>, active: ConfigTab, p: &Palette) -> impl IntoElement {
    div()
        .id("config-tabs")
        .flex()
        .flex_col()
        .gap_1()
        .w(px(104.))
        .border_r_1()
        .pr_2()
        .children(ConfigTab::ALL.into_iter().map(|tab| {
            let d = dialog.clone();
            div()
                .id(format!("config-tab-{}", tab.label()))
                .soft_button(p, active == tab)
                .child(tab.label())
                .on_click(move |_ev, _window, cx| {
                    d.update(cx, |this, cx| {
                        this.tab = tab;
                        this.error = None;
                        cx.notify();
                    });
                })
        }))
}

/// 「每日上限」页：显示当前值并用步进按钮写库。
fn daily_cap_content(
    dialog: Entity<ConfigDialog>,
    root: Entity<MinActionApp>,
    cap: u8,
    p: &Palette,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(TEXT_MD)
                .text_color(p.text)
                .child(format!("每天最多生成 {cap} 条行动")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(cap_step(
                    dialog.clone(),
                    root.clone(),
                    "cap-config-dec",
                    "−",
                    cap.saturating_sub(1).max(crate::common::DAILY_CAP_MIN),
                    p,
                ))
                .child(div().min_w(px(24.)).text_center().child(cap.to_string()))
                .child(cap_step(
                    dialog,
                    root,
                    "cap-config-inc",
                    "+",
                    cap.saturating_add(1).min(crate::common::DAILY_CAP_MAX),
                    p,
                )),
        )
}

/// 每日上限步进按钮。
fn cap_step(
    dialog: Entity<ConfigDialog>,
    root: Entity<MinActionApp>,
    id: &'static str,
    label: &'static str,
    target: u8,
    p: &Palette,
) -> impl IntoElement {
    div()
        .id(id)
        .tone_button(p, Tone::Neutral)
        .px_3()
        .child(label)
        .on_click(move |_ev, _window, cx| {
            root.update(cx, |app, cx| {
                let _ = app.set_daily_cap(target);
                cx.notify();
            });
            dialog.update(cx, |this, cx| {
                this.cap = target;
                cx.notify();
            });
        })
}

/// 「主题」页：三种偏好单选，点击即应用并持久化。
fn theme_content(
    dialog: Entity<ConfigDialog>,
    root: Entity<MinActionApp>,
    current: ThemeChoice,
    p: &Palette,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(TEXT_MD)
                .text_color(p.text)
                .child("选择界面主题，切换后立即生效并被记住。"),
        )
        .children(
            [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark]
                .into_iter()
                .map(|choice| theme_option(dialog.clone(), root.clone(), choice, current, p)),
        )
}

/// 单个主题选项：当前项高亮，点击应用主题并写回设置。
fn theme_option(
    dialog: Entity<ConfigDialog>,
    root: Entity<MinActionApp>,
    choice: ThemeChoice,
    current: ThemeChoice,
    p: &Palette,
) -> impl IntoElement {
    div()
        .id(format!("config-theme-{}", choice.as_str()))
        .soft_button(p, choice == current)
        .child(theme_label(choice))
        .on_click(move |_ev, window, cx| {
            apply_theme_choice(choice, window, cx);
            root.update(cx, |app, cx| {
                let _ = app.set_theme(choice);
                cx.notify();
            });
            dialog.update(cx, |this, cx| {
                this.theme = choice;
                this.error = None;
                this.notice = None;
                cx.notify();
            });
        })
}

/// 「存储地址」页：展示当前目录与占用，输入新目录并迁移。
fn storage_content(
    dialog: Entity<ConfigDialog>,
    path_input: Entity<InputState>,
    store: Arc<Store>,
    p: &Palette,
) -> impl IntoElement {
    let info = storage_dialog::collect_storage(store.data_dir());
    let rows = storage_dialog::storage_rows(&info);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(
            rows.into_iter()
                .map(|(label, value)| info_row(label, value, p)),
        )
        .child(field_label("新目录（迁移后数据将搬过去）", p))
        .child(Input::new(&path_input))
        .child(
            div()
                .id("config-migrate")
                .tone_button(p, Tone::Primary)
                .px_3()
                .child("迁移数据")
                .on_click(move |_ev, _window, cx| {
                    let path = path_input.read(cx).value().trim().to_owned();
                    if path.is_empty() {
                        dialog.update(cx, |this, cx| {
                            this.error = Some("请填写目标目录".to_owned());
                            cx.notify();
                        });
                        return;
                    }
                    let new_dir = PathBuf::from(path);
                    let bootstrap = dialog.read(cx).bootstrap.clone();
                    let root = dialog.read(cx).root.clone();
                    match migrate_store(store.as_ref(), &new_dir, &bootstrap) {
                        Ok((new_store, old_dir)) => {
                            root.update(cx, |app, cx| {
                                let _ = app.replace_store(new_store.clone());
                                cx.notify();
                            });
                            dialog.update(cx, |this, cx| {
                                this.store = new_store;
                                this.notice = Some(format!("数据已迁移到 {}", new_dir.display()));
                                cx.notify();
                            });
                            if let Err(error) = storage::cleanup_old_data(&old_dir) {
                                dialog.update(cx, |this, cx| {
                                    this.error = Some(error.to_string());
                                    cx.notify();
                                });
                            }
                        }
                        Err(error) => dialog.update(cx, |this, cx| {
                            this.error = Some(error);
                            cx.notify();
                        }),
                    }
                }),
        )
}

/// 调用数据层迁移并返回新连接与旧目录，供 UI 替换连接与清理。
pub(crate) fn migrate_store(
    store: &Store,
    new_dir: &Path,
    bootstrap: &Bootstrap,
) -> Result<(Arc<Store>, PathBuf), String> {
    let old_dir = store.data_dir().to_path_buf();
    let relocation = storage::relocate(store, new_dir, bootstrap).map_err(|e| e.to_string())?;
    Ok((Arc::new(relocation.store), old_dir))
}

#[cfg(test)]
mod tests;
