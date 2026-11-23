# 最小行动（ngy-min-action）

用 Rust + [gpui-kit](https://crates.io/crates/gpui-kit) 写的 Windows 桌面应用：把长期方向拆成
「目标 → 分类 → 任务」，每天只生成数量受限的一份最小行动清单，并按遗忘曲线安排复习。

数据存本地 SQLite，不联网；领域层、存储层与调度层不依赖界面框架，可无头测试。

## 功能

- **结构树**：左侧「全部 / 目标 / 分类」三级树，显示分类下的任务条数
- **任务管理**：新建、改名、调整频率（每天 / 每 N 天 / 每周 N 次）与预计耗时；
  点击任务卡片打开内容编辑器，可写富文本内容（链接可点击）并附图片、音频、视频
- **今日待办**：完成、跳过、撤销，写备注（左右两栏对照任务内容）
- **历史待办**：按日期倒序分页浏览，可补结算或撤销
- **每日上限**：1–20 条，默认 5 条；超出的行动顺延，顺延 7 天未处理转为过期
- **复习调度**：完成常规行动后按 `[1, 2, 4, 7, 15, 30]` 天推进，6 级毕业
- **统计**：复习窗口分项、完成率、连续达成天数、任务阶段分布
- **首选项**：每日上限、数据位置迁移、主题（跟随系统 / 浅色 / 深色）
- **单实例**：同一台机器同时只允许一个实例运行

## 环境要求

- Windows（标题栏与任务栏图标的设置走 Win32 API；其他平台有回退分支，但未作为目标平台验证）
- [rustup](https://rustup.rs/)；工具链由 `rust-toolchain.toml` 固定为 1.97，
  `rustfmt` 与 `clippy` 随工具链安装

## 快速开始

```powershell
cargo run
```

首次启动会在 `%LOCALAPPDATA%\ngy-min-action` 下创建 `data.db` 并初始化表结构。
若已有实例在运行，新实例会记录一条日志后直接退出；关闭主窗口即退出程序。

发布构建与性能测量：

```powershell
cargo build --release
pwsh scripts/perf.ps1 -Runs 20
```

## 使用流程

1. 顶部「编辑 → 新建目标」建立第一个目标
2. 在结构树选中目标，再「编辑 → 新建分类」
3. 选中分类，「编辑 → 新建任务」（默认频率「每天」）
4. 在「任务」页点击任务卡片打开内容编辑器：改名称、预计耗时与内容，增删媒体附件；
   频率走独立的频率对话框
5. 切到「今日待办」完成或跳过当天行动，需要时写备注
6. 「历史待办」里可回看并补结算
7. 「编辑 → 首选项」调整每日上限、数据位置与主题；切到「统计」页签查看复习情况

## 每日生成规则

`src/schedule/generate.rs` 的 `plan_day` 是纯函数，同一输入结果恒定：

- 常规行动在「上次生成日 + 频率间隔」到期时参与竞争，复习行动在 `next_review_on` 到期时参与
- 同一任务的复习优先于常规；候选按「类型 → 到期日 → 任务 ID」稳定排序
- 当天已有行动先占名额，剩余名额按每日上限截断，超出部分记为 `deferred`
- 同一天重复生成是幂等的（`UNIQUE(task_id, on_date)`，已生成的任务不再重复插入）

复习推进见 `src/schedule/review.rs`：完成才推进级别，跳过不推进，撤销通过
`action_settlements` 里的快照回退。

## 数据位置

默认数据目录 `%LOCALAPPDATA%\ngy-min-action`：

| 文件 / 目录 | 作用 |
| --- | --- |
| `data.db` | SQLite 数据库（当前 schema 版本 4） |
| `attachments/<行动 ID>/` | 行动图片副本 |
| `task_content/<任务 ID>/` | 任务内容的图片、音频、视频副本 |
| `location` | 自定义数据目录的绝对路径（不存在则用默认目录） |
| `app.lock` | 单实例锁 |

存储地址迁移在「首选项 → 存储地址」里完成：先在线备份数据库、复制附件，再用新库执行
`integrity_check`，通过后才写入位置配置并清理旧数据；任一步失败都会回收目标目录里的临时产物，
旧位置保持可用。

日志经 `ngy-utils-tracing` 输出：`development` 写 stderr（带文件名与行号），`production`
只写文件，stdout 不输出日志。

## 目录结构

| 路径 | 作用 |
| --- | --- |
| `src/main.rs` | 程序入口：初始化日志、记录退出码 |
| `src/run.rs` | 启动编排：单实例锁、数据目录、窗口尺寸与图标 |
| `src/app/` | gpui 界面层：根视图、结构树、四页签、工具栏与各类对话框 |
| `src/model*` | 领域类型、边界校验、主题与统计模型 |
| `src/store*` | SQLite 存储层：建表迁移、事务化读写、在线备份 |
| `src/schedule*` | 每日生成与遗忘曲线调度（纯函数） |
| `src/storage.rs` | 数据位置迁移与旧数据清理 |
| `src/attachment.rs` | 图片与媒体附件的校验、复制与删除 |
| `src/config.rs` | 启动目录与数据目录解析 |
| `src/palette.rs` / `src/app/style.rs` | 随主题自适应的取色板与视觉令牌 |
| `tests/` | 无头验收测试，按领域模块镜像 |
| `benches/plan_day.rs` | 200 任务规模下 `plan_day` 的微基准 |
| `scripts/perf.ps1` | 启动耗时、空闲 CPU、常驻内存的 p95 测量 |

## 开发

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo bench --bench plan_day
```

在 Windows 上跑构建或测试前先停掉正在运行的实例，否则可执行文件被占用。

CI（`.github/workflows/ci.yml`）在提交与 PR 上执行：

| Job | 内容 |
| --- | --- |
| `cargo fmt --check` | 格式 |
| `cargo clippy -D warnings` | lint + `cargo machete` 未使用依赖 |
| `cargo test` | 全部测试 |
| `Constitution governance scan` | TODO 归属、禁止 `#[allow(dead_code)]`、单文件 700 行上限 |
| `cargo audit` | 依赖漏洞 |
| `Secret scan` | 机密与令牌扫描 |

## 已知限制

- 数据位置迁移目前只搬运 `data.db` 与 `attachments/`，`task_content/` 尚未纳入
- 每日提示的判定（`src/notify.rs`）与提示记录已实现，但尚未接入界面
- 任务暂停（`TaskStatus::Paused`）只有数据层支持，界面暂无入口

## 协作约定

- 开发与提交规则见 [`AGENTS.md`](AGENTS.md)
- 项目治理原则（文件规模、测试先行、日志与安全基线等）见
  [`.specify/memory/constitution.md`](.specify/memory/constitution.md)
