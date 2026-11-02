---
description: "Task list for 001-min-action-core"
---

# Tasks: 最小行动管理系统（每日最小行动闭环）

**Input**: Design documents from `/specs/001-min-action-core/`

**Prerequisites**: plan.md（必需）、spec.md（用户故事）、research.md、data-model.md、contracts/

**Tests**: 必需。宪法第 III 条（NON-NEGOTIABLE）要求测试先行——每个功能任务前必须先有能观察到失败的测试。

**Organization**: 任务按用户故事分组，每个故事可独立实现、独立测试、独立交付。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、无未完成依赖）
- **[Story]**: 所属用户故事（US1 / US2 / US3 / US4）
- 每条任务都带确切文件路径

## Path Conventions

单 crate 单项目：`src/` 与 `tests/` 在仓库根，模块布局见 [plan.md](./plan.md) 的 Source Code 小节。

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 依赖、模块骨架、常量与领域基础类型

- [ ] T001 在 `Cargo.toml` 添加依赖：`eframe = "0.37"`、`tokio = { version = "1.54", features = ["rt-multi-thread", "time", "sync", "macros"] }`、`rusqlite = { version = "0.41", features = ["bundled"] }`、`chrono = { version = "0.4", features = ["clock"] }`；保留现有 `anyhow` / `tracing` / `ngy-utils-tracing`；确认 `cargo machete` 不因新增未使用依赖失败（依赖与首个使用它们的任务同批提交）
- [ ] T002 新建 `src/lib.rs`，声明 `mod app; mod clock; mod common; mod model; mod notify; mod schedule; mod store;` 并按 plan.md 布局导出（模块根只放 `mod` 声明与再导出，不就地定义类型）；同步在 `src/main.rs` 改为引用 `ngy_min_action` crate
- [ ] T003 [P] 在 `src/common/constants.rs` 增加常量：遗忘曲线间隔序列 `[1, 2, 4, 7, 15, 30]`、默认每日上限 `5`、每日上限允许范围 `1..=20`、每任务每日上限 `1`、`deferred` 保留天数 `7`、应用数据目录名，全部带 rustdoc 说明「为什么是这个数」
- [ ] T004 [P] 在 `src/model.rs` 定义身份类型 `GoalId` / `CategoryId` / `TaskId` / `ActionId`（newtype，注释说明防混用理由）与错误类型 `DomainError`、`StoreError`、`NameViolation`，按 `contracts/domain-api.md` 第 6 节实现可执行的 `Display` 文案（哪个输入 / 期望什么 / 如何修正）

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 时钟、数据库骨架、测试夹具、窗口外壳。任何一个用户故事开工前必须完成

**⚠️ CRITICAL**: 本阶段完成前，任何用户故事都不能开始

- [ ] T005 [P] 在 `src/clock.rs` 实现 `Clock` trait（`fn today(&self) -> NaiveDate`）、`SystemClock`（基于 `chrono::Local`）与测试用固定时钟；文档注释写明「业务层禁止直接调用 `Local::now()`」的契约
- [ ] T006 在 `src/store/schema.rs` 实现建表与迁移：`goals`、`categories`、`tasks`、`actions`、`review_progress`、`reminder_state`、`settings`、`action_settlements` 八张表，含 `UNIQUE(goal_id, name)`、`UNIQUE(task_id, on_date)`、`CHECK (id = 1)`、各级外键 `ON DELETE CASCADE`，启用 `journal_mode=WAL` 与 `foreign_keys=ON`（约束逐项对照 `data-model.md`）
- [ ] T007 在 `src/store.rs` 实现 `Store::open(path)` 与迁移调用、`Arc<Mutex<Connection>>` 单一连接、`spawn_blocking` 包装与每方法一个事务的边界；路径不可写与 schema 版本超前时返回 `StoreError` 并给出可执行说明
- [ ] T008 [P] 在 `tests/common/mod.rs` 建立共享夹具：临时目录 + 临时数据库 + 固定时钟，供各 `tests/*_test.rs` 用 `mod common;` 引入（测试文件之间不互相 `use`）
- [ ] T009 在 `src/app.rs` 实现界门外壳：`eframe` 启动、主窗口最小尺寸、状态条占位与左右分栏骨架；确认关闭主窗口即退出程序、最小化后进程继续存活
- [ ] T010 在 `src/run.rs` 与 `src/main.rs` 完成编排：按 `research.md` D7 在数据目录对 `app.lock` 取独占文件锁（取不到则聚焦既有实例并退出）、在后台线程启动 tokio 运行时、把运行时句柄与 `Store` 传入界面；保持 `main` 中「错误只记一次」的边界，日志初始化失败仍只写 stderr

**Checkpoint**: 外壳可启动、数据库可打开、测试夹具可用 —— 用户故事可以开工

---

## Phase 3: User Story 1 - 建立目标、分类与任务三层结构 (Priority: P1) 🎯 MVP

**Goal**: 用户可以创建、查看、重命名、删除目标与分类，并在分类下创建带频率的任务

**Independent Test**: 从空数据依次建出「学习英语 → 口语 → 每周学习一个连读」，逐层查看归属正确；重复建同名分类被拒且列表不变；删除目标时确认影响范围后级联清理

### Tests for User Story 1 ⚠️

> 先写测试并运行到失败，再写实现（宪法 III）

- [ ] T011 [P] [US1] 在 `tests/store_test.rs` 编写目标/分类/任务用例：同名分类在同一目标下被拒、跨目标可重名、名称去空白后为空被拒、含控制字符被拒、删除目标级联清理、暂停任务不产生当日常规行动资格

### Implementation for User Story 1

- [ ] T012 [US1] 在 `src/model.rs` 补 `Goal`、`Category`、`Task`、`Frequency`、`TaskStatus` 与名称校验（目标/分类 1..=80 字符、任务 1..=120 字符、去首尾空白后非空、拒绝换行与制表符等控制字符），校验在构造时完成，进入存储层的名字已合法
- [ ] T013 [US1] 在 `src/store/goal.rs` 实现目标与分类的创建、重命名、删除、按目标列出分类；`UNIQUE(goal_id, name)` 冲突转换为 `DomainError::DuplicateName` 并指出重复的是哪个名称
- [ ] T014 [US1] 在 `src/store/task.rs` 实现任务创建、重命名、删除、按分类列出，以及 `every_n_days` 的 n ∈ 1..=365 与 `estimated_minutes` 1..=480 的边界校验
- [ ] T015 [US1] 在 `src/app/main_view.rs` 实现左栏目标/分类树与右栏任务列表，含新建表单与内联错误文案（错误紧贴输入项，按「哪个输入 / 期望 / 如何修正」组织）
- [ ] T016 [US1] 在 `src/app.rs` 接线：启动加载树、提交后刷新、删除前展示将被一并删除的分类/任务/未完成行动数量并要求确认

**Checkpoint**: 三层结构可独立跑通并要求删除确认 —— MVP 达成

---

## Phase 4: User Story 2 - 每天自动得到一份可执行的最小行动清单 (Priority: P1)

**Goal**: 按频率自动生成当天行动，受全局上限与每任务上限双重约束，超出部分顺延且可见

**Independent Test**: 8 个每天任务 + 上限 5 → 只列 5 条并显示「已顺延 3 条」；重复查看同一天结果完全一致；暂停任务不出现

### Tests for User Story 2 ⚠️

- [ ] T017 [P] [US2] 在 `tests/schedule_test.rs` 编写用例：`daily` / `every_n_days` / `weekly_n` 的到期判定（含 `⌊7/n⌋` 退化）、同一任务当天最多一条、复习优先于常规、超上限截断与顺延计数、同一输入两次调用结果一致

### Implementation for User Story 2

- [ ] T018 [P] [US2] 在 `src/schedule/frequency.rs` 实现 `is_due(task, today)`，覆盖 `last_generated_on` 为空、`daily`、`every_n_days`、`weekly_n = ⌊7 / n⌋` 四种情形
- [ ] T019 [US2] 在 `src/schedule/generate.rs` 实现纯函数 `plan_day(input)`：候选收集（常规 + 到期复习）→ 每任务最多一条 → 排序（复习优先，同类型按到期日早者优先，再按任务创建时间稳定排序）→ 按上限截断 → 返回写入项与顺延数量
- [ ] T020 [US2] 在 `src/schedule.rs` 提供调度门面与 `PlanInput` / `PlanOutput` 类型，写清「同一输入结果恒等」的契约注释
- [ ] T021 [US2] 在 `src/store/action.rs` 实现 `generate_day(today, daily_cap)`：以 `INSERT ... ON CONFLICT DO NOTHING` 依赖 `UNIQUE(task_id, on_date)` 保证幂等，超上限项落库为 `deferred`，已存在的行动保持原状态，整笔在一个事务内
- [ ] T022 [US2] 在 `src/store/reminder.rs` 实现 `settings` 读写：`daily_cap` 默认 5、允许 1..=20，非法值返回可执行错误
- [ ] T023 [US2] 在 `src/app/today_view.rs` 实现今日清单：日期、行动行（「分类名 · 行动文案」+ 复习级别标注）、状态条（总数 + 顺延数，顺延数 > 0 时不可隐藏）、空状态引导文案
- [ ] T024 [US2] 在 `src/app.rs` 接线今日视图：跨日切换后重新生成并刷新，`deferred` 超过 7 天转 `expired` 并写一条 `warn` 日志（级别字段结构化）

**Checkpoint**: US1 + US2 均独立可用，每天能看到受上限约束的清单

---

## Phase 5: User Story 3 - 完成行动后由遗忘曲线接管复习 (Priority: P2)

**Goal**: 完成后按 `[1,2,4,7,15,30]` 天安排复习；跳过不推进；可查看进度、可重置、可撤销

**Independent Test**: 完成一条行动 → 进度为第 1 级、下次为次日；次日清单出现「第 1 级复习」；走完 6 级标记已巩固；跳过与重置不污染序列

### Tests for User Story 3 ⚠️

- [ ] T025 [P] [US3] 在 `tests/review_test.rs` 编写用例：第 1 级为 +1 天、第 k 级为 +`INTERVALS_DAYS[k]`、走完序列置 `graduated`、跳过不推进、重置于未开始、撤销回退到快照、`level` 越界返回 `ScheduleError::InvalidLevel`

### Implementation for User Story 3

- [ ] T026 [US3] 在 `src/schedule/review.rs` 实现 `advance_review(progress, completed_on, intervals)` 与回滚所需的前置快照计算
- [ ] T027 [US3] 在 `src/store/action.rs` 实现 `settle_action`（完成 / 跳过）与 `undo_last_settlement`：完成推进复习、跳过次日重新出现，每次结算把变更前的复习快照写入 `action_settlements`，撤销时回放最近一条
- [ ] T028 [US3] 在 `src/app/today_view.rs` 增加行动行的「完成」「跳过」与撤销入口，并在复习行动上显示当前级别
- [ ] T029 [US3] 在 `src/app/main_view.rs` 增加任务行的复习进度展示（当前级别 + 下次复习日 + 是否已巩固）与「重置复习进度」入口

**Checkpoint**: 记忆闭环可验证，三个故事互不破坏

---

## Phase 6: User Story 4 - 常驻运行，每天弹出一次最前端提示 (Priority: P3)

**Goal**: 程序常驻，跨多天不重启也每天弹一次置顶窗口；关窗即当天已提示；睡眠恢复补发；重启不重复

**Independent Test**: 同一天只弹一次；关窗后重启当天不再弹；无待办不弹；改日期到次日后 5 秒内弹出且不抢焦点

### Tests for User Story 4 ⚠️

- [ ] T030 [P] [US4] 在 `tests/notify_test.rs` 编写用例：当天首次满足条件弹一次、`last_notified_on == today` 时不弹、`last_viewed_on == today` 时不弹、待办为 0 时不弹、跨日后重新满足、模拟挂起恢复补发仍只一次（全部经固定时钟驱动）

### Implementation for User Story 4

- [ ] T031 [US4] 在 `src/notify.rs` 实现纯函数 `should_notify(state, today, pending)` 与后台跨日检测任务：tokio `interval` 每 2 秒比较本地日期，日期变化即触发新一天生成与提示判定（`research.md` D5）
- [ ] T032 [US4] 在 `src/app/popup.rs` 实现独立置顶视口：层级始终置顶、内容为当天未完成数量、不主动夺取键盘焦点、同一天只允许一个提示窗口实例
- [ ] T033 [US4] 在 `src/run.rs` 与 `src/app.rs` 接线弹窗流程：判定 → 弹窗 → 弹成功后写回 `last_notified_on` → 「查看今日清单」聚焦主窗口并切到今日视图；程序启动时同样按此规则判定

**Checkpoint**: 四个故事全部独立可用

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 性能证据、日志一致性、文档与门禁

- [ ] T034 [P] 在 `benches/plan_day.rs` 用 `criterion` 写 `plan_day` 在 200 任务输入下的微基准（与第一个基准同批把 `criterion` 加入 `[dev-dependencies]`）
- [ ] T035 [P] 编写 `scripts/perf.ps1`：连续 20 次测量启动到可交互、跨日弹窗延迟、交互响应，并用 `Get-Process` 采样 `WorkingSet` 与空闲 CPU，输出 p95；结果填入 issue #1 的表格
- [ ] T036 [P] 更新 `README.md`：说明这是 Windows 桌面常驻程序（`cargo run` 启动、关闭主窗口即退出、最小化继续驻留），替换现有 CLI 描述
- [ ] T037 统一结构化日志：在 `src/store/*.rs`、`src/notify.rs`、`src/app.rs` 的操作边界补齐 `op` / `result` / `elapsed_ms` 字段，错误只在 `src/main.rs` 边界记一次，production 只写 JSON 文件且 stdout/stderr 洁净（宪法 VII）
- [ ] T038 跑完整门禁：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`，并按 `quickstart.md` 手工走完五个验收场景
- [ ] T039 按阶段拆分提交并写 PR 描述：说明为什么改、链接 `specs/001-min-action-core/plan.md` 与 `spec.md`、列出原则 V 预算例外与 `TODO(#1)`、附单开发者仓库条款的自我证明

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖，可立即开始
- **Foundational (Phase 2)**: 依赖 Setup，**阻塞所有用户故事**
- **User Stories (Phase 3+)**: 均依赖 Foundational 完成
- **Polish (Phase 7)**: 依赖四个故事全部完成

### User Story Dependencies

- **US1 (P1)**: Foundational 后可独立开始，无跨故事依赖
- **US2 (P1)**: Foundational 后开始；生成需要 US1 的任务数据，但可用 `tests` 夹具直接构造数据，因此可独立测试
- **US3 (P2)**: 依赖 US2 产出的行动才能结算，属数据依赖而非实现依赖；测试同样用夹具
- **US4 (P3)**: 依赖 US2 的当日清单以取得未完成数量

### Within Each User Story

- 测试先写并运行到失败，再写实现（宪法 III，不可跳过）
- 模型与校验先于存储，存储先于界面接线
- 同文件的任务不得并行（如 T021 与 T027 都在 `src/store/action.rs`）

### Parallel Opportunities

- T003 / T004 可并行；T005 / T008 可并行
- T011 与 T012 可并行（不同文件）
- T017 / T018 可并行（不同文件）
- T025 / T026 可并行
- T030 / T031 可并行
- T034 / T035 / T036 可并行

---

## Parallel Example: User Story 4

```bash
# 测试与实现分处不同文件，可同时开工：
Task: "tests/notify_test.rs 编写每日提示用例（先失败）"
Task: "src/notify.rs 实现 should_notify 与跨日轮询任务"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. 完成 Phase 1: Setup
2. 完成 Phase 2: Foundational（阻塞项）
3. 完成 Phase 3: US1
4. **停下来验证**：单独跑 US1 的独立测试与手工场景 1
5. 可交付：一个能长期维护的目标/分类/任务清单桌面程序

### Incremental Delivery

1. Setup + Foundational → 地基就绪
2. US1 → 验证 → 可交付（MVP）
3. US2 → 验证 → 每天的最小行动清单
4. US3 → 验证 → 记忆闭环
5. US4 → 验证 → 每日提醒

### 提交策略

按 `AGENTS.md` 的「提交策略」：每完成一个阶段或一个可独立验证的批次即提交，规格/设计产物与代码改动分开，不为凑绿加 lint 抑制。

---

## Notes

- `[P]` 指不同文件且无未完成依赖；同文件任务一律不并行
- 每条测试任务都要求「先看到失败」，未失败的测试不算数（宪法 III）
- 文件名上限 700 行、600 行预警（宪法 I）：`src/store/action.rs` 与本文件里被多次修改的 `src/app.rs` 最可能先撞线，触线即拆
- 任何 TODO 只能写成 `TODO(#<issue>): <说明>`，当前唯一允许的是 `TODO(#1)`（性能预算收尾）
- 覆盖的规格条款见各阶段标题与任务描述，逐项对照 `spec.md` 的 FR 编号
