//! 启动编排：单实例锁、数据目录、后台运行时与窗口启动（gpui-kit）。

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tracing::{info, warn};

use gpui_kit::{
    App, AppContext, Bounds, Pixels, Size, WindowBounds, WindowOptions, point, px, size,
};

use ngy_min_action::app::{MinActionApp, WINDOW_TITLE, apply_theme_choice};
use ngy_min_action::clock::{Clock, SystemClock};
use ngy_min_action::common::elapsed_ms;
use ngy_min_action::config::Bootstrap;
use ngy_min_action::store::Store;

/// 业务逻辑入口。
///
/// 出错时返回错误而不是自行记录，由 `main` 在边界处统一记录一次。
pub(crate) fn run() -> Result<()> {
    let started = Instant::now();
    let bootstrap = Bootstrap::system();
    let Some(_instance) = acquire_instance(&bootstrap.lock_path())? else {
        warn!(
            op = "startup",
            result = "already_running",
            elapsed_ms = elapsed_ms(started),
            "程序已在运行，本次启动退出"
        );
        return Ok(());
    };

    let directory = bootstrap.data_dir();
    let store = Store::open(&directory.join("data.db")).context("打开数据文件失败")?;
    info!(
        op = "startup",
        data_dir = %directory.display(),
        lock = %bootstrap.lock_path().display(),
        "数据位置已解析"
    );
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    let app = MinActionApp::with_bootstrap(store, clock, bootstrap)?;
    info!(
        op = "startup",
        result = "ok",
        elapsed_ms = elapsed_ms(started),
        "界面已就绪"
    );

    let theme = app.initial_theme();
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let window_bounds = primary_window_bounds(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                ..Default::default()
            },
            cx,
            |window, cx| {
                window.set_window_title(WINDOW_TITLE);
                apply_theme_choice(theme, window, cx);
                cx.new(|_| app)
            },
        )
        .expect("打开主窗口失败");
    });
    Ok(())
}

/// 依据显示器可用区域计算初始窗口尺寸：取舒适尺寸与可用区域（留边距）中的较小值。
fn fitted_window_size(available: Size<Pixels>) -> Size<Pixels> {
    const PREFERRED_WIDTH: f32 = 1180.0;
    const PREFERRED_HEIGHT: f32 = 720.0;
    const MARGIN: f32 = 48.0;

    let usable_width = available.width.as_f32() - MARGIN;
    let usable_height = available.height.as_f32() - MARGIN;
    if usable_width <= 0.0 || usable_height <= 0.0 {
        return size(px(PREFERRED_WIDTH), px(PREFERRED_HEIGHT));
    }
    size(
        px(PREFERRED_WIDTH.min(usable_width)),
        px(PREFERRED_HEIGHT.min(usable_height)),
    )
}

/// 在主显示器工作区内居中放置初始窗口，避免窗口高于屏幕或被任务栏遮挡。
fn primary_window_bounds(cx: &App) -> WindowBounds {
    let visible = cx.primary_display().map(|display| display.visible_bounds());
    let window_size = visible
        .map(|bounds| fitted_window_size(bounds.size))
        .unwrap_or_else(|| fitted_window_size(size(px(0.), px(0.))));
    match visible {
        Some(available) => {
            let origin = point(
                px(available.origin.x.as_f32()
                    + (available.size.width.as_f32() - window_size.width.as_f32()) / 2.0),
                px(available.origin.y.as_f32()
                    + (available.size.height.as_f32() - window_size.height.as_f32()) / 2.0),
            );
            WindowBounds::Windowed(Bounds::new(origin, window_size))
        }
        None => WindowBounds::centered(window_size, cx),
    }
}

/// 取单实例锁。返回 `None` 表示已有实例在运行。
fn acquire_instance(path: &Path) -> Result<Option<File>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("创建启动目录失败")?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .context("打开实例锁文件失败")?;

    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error).context("获取实例锁失败"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_window_size_keeps_preferred_on_large_display() {
        let window = fitted_window_size(size(px(1920.), px(1080.)));
        assert_eq!(window.width.as_f32(), 1180.0);
        assert_eq!(window.height.as_f32(), 720.0);
    }

    #[test]
    fn fitted_window_size_clamps_to_short_display() {
        let window = fitted_window_size(size(px(1366.), px(700.)));
        assert_eq!(window.width.as_f32(), 1180.0);
        assert_eq!(window.height.as_f32(), 652.0);
    }

    #[test]
    fn fitted_window_size_never_exceeds_available_area() {
        let window = fitted_window_size(size(px(400.), px(300.)));
        assert_eq!(window.width.as_f32(), 352.0);
        assert_eq!(window.height.as_f32(), 252.0);
    }

    #[test]
    fn fitted_window_size_falls_back_without_display() {
        let window = fitted_window_size(size(px(0.), px(0.)));
        assert_eq!(window.width.as_f32(), 1180.0);
        assert_eq!(window.height.as_f32(), 720.0);
    }
}
