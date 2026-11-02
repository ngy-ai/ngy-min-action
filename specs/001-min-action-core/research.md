# Phase 0 Research: 最小行动管理系统

**Feature**: `001-min-action-core` | **Date**: 2026-11-02

本文件只回答「为什么选它、还考虑过什么」，技术细节留给 `plan.md` 与 `contracts/`。

## D1. GUI 框架：eframe / egui 0.37

**Decision**: 使用 `eframe` 0.37（内含 `egui` 0.37）承载主界面与置顶提示窗口。

**Rationale**:

- 纯 Rust、即时模式，不需要前端资产、Node 工具链或资源打包步骤，符合宪法 VI「默认单 crate、单部署单元」。
- 原生支持第二视口（deferred viewport）与窗口层级设置：置顶提示窗口可作为独立视口弹出，不需要自绘无边框窗口。
- 产物是单个可执行文件，Windows 分发时无需安装运行时。
- 与 `tokio` 的协作是已知模式（事件循环独占主线程，运行时跑后台线程），不存在需要绕过的事件循环冲突。

**Alternatives considered**:

- **Tauri 2**：置顶窗口、托盘、系统通知都开箱可用，但它要求前端工程（HTML/JS + 打包），引入第二套工具链与 IPC 边界；宪法 VI 的简单性判据下不划算，且本 feature 的界面复杂度（三层列表 + 今日清单 + 弹窗）远未到需要 Web UI 的程度。
- **Slint**：声明式 DSL 的可维护性更好，但需要一个额外的编译期代码生成步骤，且托盘与置顶这类桌面细节仍需自建，收益不足以抵消新增心智负担。

## D2. 异步运行时：tokio 1.54 跑在后台线程，不接管主线程

**Decision**: 主线程归 `eframe` 的事件循环；`tokio` 多线程运行时在独立后台线程启动，负责跨日检测、数据库任务编排与延迟动作。两边通过通道通信，UI 侧用 `Context::request_repaint` 被动唤醒。

**Rationale**:

- Windows 的 `winit` 消息循环必须在主线程，任何「把 `block_on` 塞进 `update()`」的写法都会冻住界面。
- 反向也不成立：把 GUI 放进 tokio 任务里会破坏事件循环的线程归属。
- 用通道 + 重绘请求的模式，UI 与异步层之间只有消息，没有共享可变状态，避免锁竞争，也便于测试。

**Alternatives considered**:

- **在 `update()` 里 `block_on`**：实现最短，但一次慢查询就掉帧，且与 FR-025「不抢占、不卡界面」冲突。
- **纯 `std::thread`**：够用，但用户已明确要求用 tokio 管理并发，且 tokio 的 `time` 与任务取消能力在跨日调度上更省事。

## D3. 存储：SQLite + `rusqlite` 0.41（bundled），同步调用走 `spawn_blocking`

**Decision**: 单文件 SQLite 数据库，驱动选 `rusqlite` 0.41 并开启 `bundled` 特性；所有数据库调用在 `tokio::task::spawn_blocking` 中执行，共享连接用 `Arc<Mutex<Connection>>` 串行化。

**Rationale**:

- FR-011 的幂等与 FR-013 的截断都要求真正的事务语义，纯文件或 JSON 需要自己实现原子写入，成本高于直接上 SQLite。
- `bundled` 把 SQLite 编进二进制，Windows 上不需要额外的 `sqlite3.dll`，符合「单文件分发」。
- 本地单用户、数据量在万行以内，用异步驱动换来的并发收益为零；`spawn_blocking` 已足够把阻塞挡在异步线程之外。

**Alternatives considered**:

- **`sqlx` 0.8 + SQLite**：全异步、与 tokio 天然契合，但依赖树大、编译时间显著上升，且 `query!` 宏需要构建期数据库或离线缓存文件，会额外引入构建步骤。对本项目是纯粹的复杂度税。
- **纯文件（JSON/TOML）**：无事务、无唯一约束，幂等要靠自己加锁实现，数据一损坏就全丢（对应 spec 的「数据文件损坏」边界）。

## D4. 时间与时钟：`chrono` 0.4 + 可注入 `Clock`

**Decision**: 业务代码不直接读系统时间，统一从 `Clock` 获取「今天」与「当前时刻」；生产实现为 `SystemClock`（`chrono::Local`），测试实现为可控时钟。

**Rationale**: 遗忘曲线的第 1/2/4/7/15/30 天、跨日弹窗、连续多日不重启等场景，靠真实时间无法在测试里覆盖。宪法 VI 明确「为在测试中替换实现而引入 trait」属于正当理由，这条 `Clock` 不违反 YAGNI。

**Alternatives considered**: 直接调用 `Local::now()`——测试只能靠真的等待，不可行。

## D5. 跨日检测：2 秒轮询本地日期，而不是精确 `sleep` 到 00:00

**Decision**: 后台任务每 2 秒比较一次本地日期与「上次见到的日期」，一旦变化就触发新一天的生成与提示。

**Rationale**:

- SC-010 要求跨日后 5 秒内弹窗，2 秒轮询留足余量。
- 精确 `sleep(到下一个零点)` 在两种真实场景下会失效：电脑睡眠跨越零点（定时器不保证按时唤醒）、用户手动改系统时间。轮询对这两种情况天然免疫，直接满足 FR-029（挂起恢复后补发）。
- 一次 `Local::now()` 的代价是系统调用级别，2 秒一次对空闲 CPU 预算（p95 < 1%）无影响。

**Alternatives considered**:

- **精确定时到零点**：正常路径更"优雅"，但两种失效场景都需要额外的唤醒补偿逻辑，总复杂度更高。
- **监听系统电源事件（Windows `WM_POWERBROADCAST`）**：能精确感知唤醒，但绑定平台窗口过程，且仍解决不了改时钟的问题。

## D6. 置顶弹窗的形态

**Decision**: 提示窗口是独立视口，层级设为「总是置顶」，尺寸固定且较小，只展示当天未完成数量与一个「查看今日清单」入口；弹出时不主动抢键盘焦点。

**Rationale**: FR-025 明确要求覆盖在其他窗口之上但不打断输入。独立视口让弹窗与主窗口互不干扰，主窗口最小化时弹窗仍可独立出现。

**Alternatives considered**: 在主窗口内叠一个模态层——主窗口被最小化或关闭后就无法提示，不满足「常驻后每天提示」。

## D7. 单实例约束

**Decision**: 启动时在数据目录对 `app.lock` 取独占文件锁（标准库文件锁，Rust 1.89 起稳定）；取不到锁就认为已有实例在跑，直接退出并给出可执行提示。

**Rationale**: 若允许双开，同一台机器会有两个进程各自判定「今天没提示过」，FR-026（每天最多弹一次）立刻失效，同时 SQLite 出现写竞争。这不是新增功能，是让 FR-026 可被满足的必要条件。

**Alternatives considered**: 命名互斥体（Windows API）——平台绑定，且文件锁已足够。

## D8. 托盘与开机自启

**Decision**: 不实现托盘图标，不实现开机自启。

**Rationale**: spec 只要求「启动后常驻 + 每天弹窗」，托盘与自启都没有当前消费者（宪法 VI）。主窗口最小化即视为「退到后台」，关闭主窗口即退出程序——这一点会写在 `contracts/ui-contract.md` 里，避免行为含混。

## D9. 性能测量手段

**Decision**: 核心调度（生成、截断、复习推进）用 `criterion` 微基准；启动到可交互、跨日弹窗延迟用固定脚本重复 20 次取 p95；内存用宪法已指定的 Windows `Get-Process` 采样 `WorkingSet`。`hyperfine` 不适用于 GUI 进程，启动测量由脚本承担。

**Rationale**: 宪法 V 指定了 `hyperfine`（端到端 CLI）与 `criterion`（微基准）。桌面程序的端到端测量没有 CLI 等价物，用「同一脚本、同一机器、重复 20 次、取 p95」保持同等的可复现性。

**Note**: 具体命令与被测指标见 `plan.md` 的复杂度追踪表；收尾动作挂在 issue #1。
