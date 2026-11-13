//! 启动编排：单实例锁、数据目录、后台运行时与窗口启动（gpui-kit）。

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tracing::{info, warn};

use gpui_kit::{AppContext, WindowOptions};

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

    let app = MinActionApp::new(store, clock)?;
    info!(
        op = "startup",
        result = "ok",
        elapsed_ms = elapsed_ms(started),
        "界面已就绪"
    );

    let theme = app.initial_theme();
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title(WINDOW_TITLE);
            apply_theme_choice(theme, window, cx);
            cx.new(|_| app)
        })
        .expect("打开主窗口失败");
    });
    Ok(())
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
