//! gpui-kit 根视图（迁移骨架）。
//!
//! 数据层（`Store` / `Model` / `Attachment`）不变，以 `Arc<Store>` 注入。后续阶段把结构树、
//! 三页签、对话框等逐个从 egui 平移到 gpui-kit；本文件先搭一个可启动、可渲染的骨架，确保
//! 工具链打通与门禁可运行，再在 Phase 1–4 逐块补回功能。

use std::sync::Arc;

use anyhow::Result;
use gpui_kit::base::StyledExt;
use gpui_kit::*;

use crate::clock::Clock;
use crate::config::Bootstrap;
use crate::store::Store;

/// 主窗口标题。
pub const WINDOW_TITLE: &str = "最小行动";

/// 后台 ticker 投递的事件（跨日检测由根视图消费）。
#[derive(Debug, Clone, Copy)]
pub enum TickEvent {
    /// 定时心跳。
    Tick,
}

/// 应用根视图：持有数据层与界面状态。
pub struct MinActionApp {
    #[allow(dead_code)]
    store: Arc<Store>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
    #[allow(dead_code)]
    ticks: std::sync::mpsc::Receiver<TickEvent>,
    #[allow(dead_code)]
    bootstrap: Bootstrap,
}

impl MinActionApp {
    /// 构造根视图状态；数据层已就绪，仅持有引用。
    pub fn new(
        store: Store,
        clock: Arc<dyn Clock>,
        ticks: std::sync::mpsc::Receiver<TickEvent>,
        bootstrap: Bootstrap,
    ) -> Result<Self> {
        Ok(Self {
            store: Arc::new(store),
            clock,
            ticks,
            bootstrap,
        })
    }
}

impl Render for MinActionApp {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, MinActionApp>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .items_center()
            .justify_center()
            .child("最小行动 · gpui-kit 迁移骨架")
    }
}
