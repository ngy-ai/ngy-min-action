//! 应用入口：先初始化日志，再执行业务逻辑。
//!
//! 输出通道固定分工：stdout 只放业务数据，日志一律写 stderr（`production` 只写文件）。
//! CLI 契约测试因此可以直接断言 stdout，不受日志干扰。
//!
//! 日志由 `ngy-utils-tracing` 统一配置，输出形态随应用模式变化：
//! - `development`：控制台（pretty，带文件名/行号/线程，写 stderr），详尽到单个步骤
//! - `test`：控制台（写 stderr）+ 文件（JSON）
//! - `production`：文件（JSON，按天轮转），只保留必要记录，stdout 与 stderr 均无日志

use std::process::ExitCode;
use std::time::Instant;

use ngy_min_action::common;
use ngy_utils_tracing::{InitOptions, init};
use tracing::{error, info};

mod run;

fn main() -> ExitCode {
    let started = Instant::now();

    // `InitResult` 持有文件日志的 WorkerGuard 和后台清理任务，必须存活到进程结束才不丢日志。
    let logging = match init(
        InitOptions::new(common::ENV_PREFIX)
            // 调试构建下的默认级别：项目自身 crate 用 debug，其余依赖回退到 info，可被 RUST_LOG 覆盖。
            // `CARGO_CRATE_NAME` 已经是 tracing 使用的 target 名（连字符转下划线）。
            .crates(vec![env!("CARGO_CRATE_NAME").to_string()]),
    ) {
        Ok(logging) => logging,
        // 此时订阅者尚未建立，无法用结构化日志记录，只能写 stderr 以免污染 stdout。
        Err(error) => {
            eprintln!("日志初始化失败: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    info!(
        op = "app_start",
        result = "ok",
        mode = %logging.mode,
        version = env!("CARGO_PKG_VERSION"),
        "应用启动"
    );

    // 错误只在这一层记录一次：内部各层负责返回错误，不自行打点，避免重复日志。
    let exit_code = match run::run() {
        Ok(()) => {
            info!(
                op = "app_exit",
                result = "ok",
                elapsed_ms = common::elapsed_ms(started),
                "应用退出"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!(
                op = "app_exit",
                error = %error,
                result = "error",
                elapsed_ms = common::elapsed_ms(started),
                "应用异常退出"
            );
            ExitCode::FAILURE
        }
    };

    // 显式释放，确保文件日志在进程退出前完成 flush。
    drop(logging);
    exit_code
}
