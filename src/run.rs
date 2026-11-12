//! 启动编排：单实例锁、数据目录、后台运行时与窗口启动（gpui-kit）。

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tracing::{info, warn};

use gpui_kit::{AppContext, WindowOptions};

use ngy_min_action::app::{MinActionApp, TickEvent, WINDOW_TITLE};
use ngy_min_action::clock::{Clock, SystemClock};
use ngy_min_action::common::{TICK_INTERVAL_SECS, elapsed_ms};
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
    let ticks = spawn_ticker();

    let app = MinActionApp::new(store, clock, ticks, bootstrap)?;
    info!(
        op = "startup",
        result = "ok",
        elapsed_ms = elapsed_ms(started),
        "界面已就绪"
    );

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title(WINDOW_TITLE);
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

/// 在后台线程启动 tokio 运行时，按固定间隔向界面投递 tick。
fn spawn_ticker() -> std::sync::mpsc::Receiver<TickEvent> {
    let (sender, receiver) = channel();
    let spawned = std::thread::Builder::new()
        .name("min-action-ticker".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    warn!(
                        op = "ticker",
                        result = "error",
                        error = %error,
                        "后台运行时创建失败，跨日检测不可用"
                    );
                    return;
                }
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
        });
    if let Err(error) = spawned {
        warn!(
            op = "ticker",
            result = "error",
            error = %error,
            "后台线程创建失败，跨日检测不可用"
        );
    }
    receiver
}
