//! 启动编排：单实例锁、数据目录、后台运行时与窗口启动。

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::{Context, Result};
use tracing::{info, warn};

use ngy_min_action::app::{MinActionApp, TickEvent, WINDOW_SIZE, WINDOW_TITLE};
use ngy_min_action::clock::{Clock, SystemClock};
use ngy_min_action::common::{APP_DATA_DIR, TICK_INTERVAL_SECS};
use ngy_min_action::store::Store;

/// 业务逻辑入口。
///
/// 出错时返回错误而不是自行记录，由 `main` 在边界处统一记录一次。
pub(crate) fn run() -> Result<()> {
    let directory = data_directory();
    let Some(_instance) = acquire_instance(&directory)? else {
        warn!(
            op = "startup",
            result = "already_running",
            "程序已在运行，本次启动退出"
        );
        return Ok(());
    };

    let store = Store::open(&directory.join("data.db")).context("打开数据文件失败")?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let ticks = spawn_ticker();

    let app = MinActionApp::new(store, clock, ticks)?;
    info!(op = "startup", result = "ok", "界面已就绪");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size([640.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(move |_cc| Ok(Box::new(app))),
    )
    .map_err(|error| anyhow::anyhow!("界面启动失败：{error}"))
}

/// 应用数据目录：优先用户本地数据目录，读不到时退回临时目录。
fn data_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_DATA_DIR)
}

/// 取单实例锁。返回 `None` 表示已有实例在运行。
fn acquire_instance(directory: &Path) -> Result<Option<File>> {
    std::fs::create_dir_all(directory).context("创建数据目录失败")?;
    let path = directory.join("app.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .context("打开实例锁文件失败")?;

    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error).context("获取实例锁失败"),
    }
}

/// 在后台线程启动 tokio 运行时，按固定间隔向界面投递 tick。
fn spawn_ticker() -> std::sync::mpsc::Receiver<TickEvent> {
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name("min-action-ticker".to_owned())
        .spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_time()
                .build()
            else {
                warn!(op = "ticker", result = "error", "后台运行时创建失败");
                return;
            };
            runtime.block_on(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(TICK_INTERVAL_SECS));
                loop {
                    ticker.tick().await;
                    if sender.send(TickEvent::Tick).is_err() {
                        break;
                    }
                }
            });
        })
        .ok();
    receiver
}
