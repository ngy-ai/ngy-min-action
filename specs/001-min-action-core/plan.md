# Implementation Plan: 最小行动管理系统（每日最小行动闭环）

**Branch**: `001-min-action-core` | **Date**: 2026-11-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-min-action-core/spec.md`

## Summary

做一个 Windows 桌面常驻程序：用户把长期目标拆成分类与任务，程序据此每天自动生成一份「最小行动清单」（双重上限约束），完成后按遗忘曲线 `[1,2,4,7,15,30]` 天安排复习；只要程序在运行，每天弹一次置顶窗口提示当天未完成数量。

技术路径：单 crate（lib + bin），`eframe`/`egui` 承载界面与置顶弹窗，`tokio` 跑在后台线程负责跨日检测与任务编排，`rusqlite`（bundled）落地单文件 SQLite，调度与复习均为纯函数、经可注入的 `Clock` 取时间，因此核心行为不依赖真实时钟即可测试。

## Technical Context

**Language/Version**: Rust 1.97（`rust-toolchain.toml` 固定），edition 2024

**Primary Dependencies**: `eframe` 0.37、`egui` 0.37、`tokio` 1.54（`time`、`sync`、`rt-multi-thread`）、`rusqlite` 0.41（`bundled`）、`chrono` 0.4、`tracing` 0.1、`anyhow` 1、`ngy-utils-tracing` 0.7；基准用 `criterion`（仅在写第一个基准时加入 `[dev-dependencies]`）

**Storage**: 单文件 SQLite，`journal_mode=WAL`、`foreign_keys=ON`；所有访问经单一连接（`Arc<Mutex<Connection>>`）+ `spawn_blocking` 串行化

**Testing**: `cargo test --all-targets --all-features`；调度与复习为纯函数单测，存储与提示为 `tests/*_test.rs` 集成测试；性能用 `criterion` 与外部脚本

**Target Platform**: Windows 10/11 x64（首要目标）；其他桌面平台不承诺、不测试

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 见复杂度追踪表的桌面预算草案（启动到可交互 p95 < 2s、跨日弹窗延迟 p95 < 5s、交互响应 p95 < 100ms、空闲 CPU p95 < 1%、常驻内存 p95 < 150MB、后台驻留内存 p95 < 80MB；规模 200 任务 × 90 天）

**Constraints**: 单实例运行；完全离线；文件上限 700 行（宪法 I）；跨日检测精度 ≤ 5s；弹窗不得夺取键盘焦点

**Scale/Scope**: 单用户单机；≤ 200 个任务；每天 ≤ 20 条行动（默认 5）；数据规模按 90 天滚动

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 单文件 ≤ 700 行；本次规划的模块按职责切分，预估最大文件 `store/schema.rs` 约 180 行 | PASS |
| II. 质量与可维护性 | 公共 API 写契约注释（见 `contracts/domain-api.md`）；不写 `#[allow(dead_code)]`；唯一 TODO 挂 issue #1 | PASS |
| III. 测试先行 | 调度/复习/提示均为纯函数 + `Clock` 注入，`tests/*_test.rs` 先失败后通过；存储层用临时库做集成测试 | PASS |
| IV. 接触面一致性 | 对外接触面为界面契约与公共库 API，分别由 `contracts/ui-contract.md`、`contracts/domain-api.md` 固定；错误文案统一按「哪个输入 / 期望 / 如何修正」组织 | PASS |
| V. 性能与效率 | **例外**：宪法默认预算以 CLI 单次执行为单位，常驻 GUI 进程没有等价端点，`p95 < 100ms` 与 `< 100MB` 两项无法按其原意测量。改用桌面预算草案，见复杂度追踪，收尾挂 issue #1 | **EXCEPTION（已论证）** |
| VI. 简单性与 YAGNI | 单 crate（1 lib + 1 bin target）；不引入 Repository / Service trait；`Clock` trait 是宪法明示允许的「为测试替换实现」；ID newtype 编码了防混用的不变量；不写托盘、自启、导入导出 | PASS |
| VII. 可观测性 | 沿用 `ngy-utils-tracing`：production 只写 JSON 文件、stdout/stderr 洁净；每个操作记 `op` / `result` / `elapsed_ms`；错误只在 `main` 边界记一次；不记录名称原文以外的 PII | PASS |
| VIII. 安全基线 | 名称在入口校验并转为受限类型；SQL 全程参数绑定，无字符串拼接；单实例文件锁防写竞争；无凭据与网络 | PASS |

**Phase 1 复核**: 设计完成后重新对照，八项结论不变；V 的例外范围仍限于性能预算，未被扩大。

## Project Structure

### Documentation (this feature)

```text
specs/001-min-action-core/
├── plan.md              # 本文件
├── spec.md              # 规格（已完成）
├── research.md          # Phase 0：技术选型与替代方案
├── data-model.md        # Phase 1：表结构、状态转移、校验规则
├── quickstart.md        # Phase 1：验证清单
├── contracts/
│   ├── domain-api.md    # 公共库 API 契约
│   └── ui-contract.md   # 界面行为契约
├── checklists/
│   └── requirements.md  # 规格质量检查
└── tasks.md             # Phase 2 输出（/speckit.tasks 生成）
```

### Source Code (repository root)

```text
src/
├── lib.rs                # 模块声明与再导出
├── main.rs               # 入口：日志、单实例、启动运行时与界面
├── run.rs                # 启动编排（替换现有空实现）
├── clock.rs              # Clock trait、SystemClock、测试时钟
├── model.rs              # 领域类型与边界校验（Goal/Category/Task/Action/复习）
├── store.rs              # Store 门面：open / migrate / 事务边界
├── store/
│   ├── schema.rs         # 建表与迁移
│   ├── goal.rs           # 目标与分类
│   ├── task.rs           # 任务与频率
│   ├── action.rs         # 行动、结算与撤销
│   └── reminder.rs       # 提示记录与配置
├── schedule.rs           # 调度门面
├── schedule/
│   ├── frequency.rs      # FR-008 频率判定
│   ├── generate.rs       # FR-010~015 生成、排序、截断、顺延
│   └── review.rs         # FR-018~023 遗忘曲线推进与回滚
├── notify.rs             # FR-024~031 跨日检测与弹窗判定
├── app.rs                # 界面门面：状态与消息循环
├── app/
│   ├── main_view.rs      # 主窗口：目标/分类树 + 行动列表
│   ├── today_view.rs     # 今日清单：状态条、行动行、空状态
│   └── popup.rs          # 置顶提示窗口视口
└── common/
    └── constants.rs      # 环境前缀、遗忘曲线序列、默认上限

tests/
├── clock_test.rs         # FR-016 本地日界切换
├── store_test.rs         # 唯一约束、软删除、事务回滚
├── schedule_test.rs      # 生成、截断、顺延、多日模拟
├── review_test.rs        # 遗忘曲线推进/回滚/毕业
├── notify_test.rs        # 每日一次、重启不重复、跨日补发
└── common/
    └── mod.rs            # 临时库与固定时钟的共享夹具
```

**Structure Decision**: 单 crate、两个 target（`lib` 提供可测的领域逻辑，`bin` 只做启动编排与界面装配）。模块根只放 `mod` 声明与再导出，共享类型与常量各自成文件，符合宪法「Rust 代码组织约定」。测试目录镜像 `src/<module>/`，跨模块的存储与提示测试留在 `tests/`。

## Complexity Tracking

### 宪法违规（需论证）

| Violation | Why Needed | Simpler Alternative Rejected Because |
| --- | --- | --- |
| 原则 V 的 CLI 性能预算被替换为桌面预算 | 本 feature 没有 CLI 命令，程序是常驻 GUI 进程；宪法默认预算的测量对象（单次命令执行、CLI 峰值内存）在桌面程序里没有对应端点，照字面沿用的结果是预算无法测量 | 直接沿用 `p95 < 100ms` 与 `< 100MB`：前者找不到"一次执行"的起点与终点，后者对包含 GUI 渲染与字体缓存的进程不现实。二者都会被写成无法测量的条款，恰是宪法 V 明确禁止的形态 |

### 桌面预算草案（待实测后写入宪法，收尾见 issue #1）

| 指标 | 草案阈值 | 测量命令 |
| --- | --- | --- |
| 启动到可交互（进程起 → 今日清单可点击） | p95 < 2s | 脚本连续启动 20 次取 p95，同一机器同一数据规模 |
| 跨日弹窗延迟（日期变化 → 窗口可见） | p95 < 5s | 脚本改系统日期后计时 20 次取 p95 |
| 交互响应（点击 → 界面可见变化） | p95 < 100ms | 自动化脚本注入点击，取 20 次 p95 |
| 空闲 CPU（无操作 60s 平均） | p95 < 1% | `Get-Process` 采样 |
| 常驻内存 | p95 < 150MB | `Get-Process` 采样 `WorkingSet` |
| 后台驻留内存（主窗口最小化） | p95 < 80MB | 同上 |
| 规模 | 200 任务 × 90 天 | 生成基准数据后重复上述测量 |

> `TODO(#1): 基准实测完成后，把上表数字与测量命令写进 `.specify/memory/constitution.md` 第 V 条，版本递增至 1.18.0，并删除本 TODO。`

### 非违规但需说明的设计选择

| 选择 | 理由 |
| --- | --- |
| `Clock` trait | 跨日、遗忘曲线、重启不重复提示都无法用真实时间稳定测试；宪法 VI 明示「为在测试中替换实现」是引入 trait 的正当理由 |
| ID 使用 newtype | 四个实体的 ID 底层同为整数，混用会静默写错行；包装编码了原语不具备的类型约束 |
| `deferred` 超过 7 天转 `expired` | 顺延若不设上限，连续多日不用会让待办无限堆积，与 FR-015「不倾倒积压」相悖；7 天是唯一常量，集中定义 |
| 跨日检测用 2 秒轮询而非精确定时 | 睡眠唤醒与改系统时间会让精确定时失效；轮询直接满足 FR-029，代码更少（见 `research.md` D5） |
| 界面契约靠手工验收，不做 GUI 自动化 | 宪法 III 第 4 条点名的是 CLI 命令与公共库 API，界面不在其列；桌面 GUI 的自动化测试成本远高于收益。界面行为由 `contracts/ui-contract.md` 固定，由 `quickstart.md` 的五个手工场景验收 |

## Phase 完成情况

- **Phase 0**: [research.md](./research.md) —— 9 项决策，无遗留 NEEDS CLARIFICATION
- **Phase 1**: [data-model.md](./data-model.md)、[contracts/domain-api.md](./contracts/domain-api.md)、[contracts/ui-contract.md](./contracts/ui-contract.md)、[quickstart.md](./quickstart.md)
- **Phase 2**: 待 `/speckit.tasks` 生成任务清单
