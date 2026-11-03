# Implementation Plan: 目标与分类名称的唯一性与规范化

**Branch**: `002-name-uniqueness` | **Date**: 2026-11-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-name-uniqueness/spec.md`

## Summary

把「名称」从仅做格式校验的字符串，升级为带作用域约束的标识：目标名称全局唯一，分类名称在所属目标内唯一，两者都比较去两端空白后、经 Unicode 大小写折叠的结果，且已软删除的记录仍占用名称。持久层用新增的 `name_folded` 列加整表唯一索引承担判重，架构版本从 1 升到 2，升级时按 Q1 决策清空全部业务数据。

技术路径：`validate_name` 继续负责 trim 与合法性，新增 `fold_name` 负责折叠；`store` 的四个写入方法同时写展示名与折叠名；唯一冲突由 SQLite 索引抛出，翻译成既有的 `DomainError::DuplicateName`。新增依赖仅 `caseless` 一个。

## Technical Context

**Language/Version**: Rust 1.97（`rust-toolchain.toml` 固定），edition 2024

**Primary Dependencies**: 在 001 的基础上新增 `caseless = "0.2"`（Unicode 大小写折叠）；其余不变（`eframe` 0.37、`egui` 0.37、`tokio` 1.54、`rusqlite` 0.41 bundled、`chrono` 0.4、`tracing` 0.1、`anyhow` 1、`ngy-utils-tracing` 0.7）

**Storage**: 单文件 SQLite，`journal_mode=WAL`、`foreign_keys=ON`；架构版本 1 → 2

**Testing**: `cargo test --all-targets --all-features`；新增用例集中在 `tests/store_test.rs`，迁移用例需要先构造 `user_version = 1` 的库文件

**Target Platform**: Windows 10/11 x64

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 沿用 001 的桌面预算草案，本 feature 不改变交互路径

**Constraints**: 单实例运行；完全离线；文件上限 700 行（宪法 I）；迁移必须整体事务

**Scale/Scope**: 单用户单机；目标与分类各 ≤ 100 条量级

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 改动集中在 `store/schema.rs`（预计 ~150 行）、`store/goal.rs`（预计 ~420 行）、`model.rs`（+1 个函数），均在 600 行预警线以下 | PASS |
| II. 质量与可维护性 | 四个写入方法的契约注释同步更新；不新增 lint 抑制；不引入 TODO | PASS |
| III. 测试先行 | 先写重名、折叠、trim、删除后重建、迁移五组用例并观察失败，再改实现 | PASS |
| IV. 接触面一致性 | `DuplicateName` 文案由「{scope} 下已存在名称」改为「{scope}：已存在名称」，`scope` 改为完整句子；属接触面文字变更，已在复杂度追踪登记并附迁移说明 | PASS（附变更说明） |
| V. 性能与效率 | 判重由全表扫描变为索引查找，写入多维护一列与一个索引；单用户低频写入，不触及交互预算。本 feature 不新增需要测量的端点 | PASS |
| VI. 简单性与 YAGNI | 新增依赖 1 个（`caseless`，编码标准库不具备的折叠规则）；新增列 2 个；不引入仓库层、不新增 trait | PASS |
| VII. 可观测性 | 迁移记一条 `op = "store_migrate"`、`from` / `to` / `result` / `elapsed_ms`；错误只在 `main` 边界记一次 | PASS |
| VIII. 安全基线 | 迁移的 `DROP` 列表在代码内固定，不接受外部输入；SQL 全程参数绑定；名称仍走 `validate_name` 的边界校验 | PASS |

**Phase 1 复核**: 设计完成后重新对照，八项结论不变；IV 的文字变更已写入 `contracts/domain-api.md` 的兼容性小节。

## Project Structure

### Documentation (this feature)

```text
specs/002-name-uniqueness/
├── plan.md              # 本文件
├── spec.md              # 规格（已完成）
├── research.md          # Phase 0：折叠方案、索引方案、迁移方案、文案形态
├── data-model.md        # Phase 1：表结构变更、索引、迁移步骤
├── contracts/
│   └── domain-api.md    # Phase 1：四个写入方法的契约与错误文案
├── quickstart.md        # Phase 1：六个手工验收场景
└── checklists/
    └── requirements.md  # 规格质量检查
```

### Source Code (repository root)

```text
src/
├── model.rs             # + fold_name；DuplicateName 文案模板调整
├── store/
│   ├── schema.rs        # SCHEMA_VERSION=2；DDL 增 name_folded 与两个唯一索引；apply 改造为版本分支 + 迁移事务
│   └── goal.rs          # create_goal / rename_goal 写 folded 并翻译唯一冲突；create_category / rename_category 同步
tests/
└── store_test.rs        # 重名、折叠、trim、删除后重建、迁移五组用例
Cargo.toml               # + caseless
```

**Structure Decision**: 不改模块边界。名称折叠属于领域规则，放 `model.rs`；索引与迁移属于持久层，放 `store/`。`rename_category` 已有的唯一冲突翻译逻辑保留，只调整 `scope` 取值。

**测试夹具**: 迁移用例需要独立构造旧库，放 `tests/store_test.rs` 内的局部辅助函数，仍遵循 001「夹具内联、不建 `tests/common/`」的决定。

## Complexity Tracking

### 宪法违规（需论证）

无。

### 破坏性变更（需迁移说明）

| 变更 | 影响范围 | 迁移说明 |
| --- | --- | --- |
| 唯一性覆盖已删除记录 | 001 中「删除分类后可重建同名」的行为与测试 | 删除目标或分类后，同名不可重建。001 的 `soft_deleted_category_disappears_from_lists` 用例需改为反向断言 |
| `DuplicateName` 文案改为冒号连接 | 界面错误提示、既有文案断言 | 行为不变（拒绝 + 不落行）。新文案见 `contracts/domain-api.md` |
| 架构版本 1 → 2 时清空全部业务数据 | 用户既有数据 | 一次性、不可逆、无内置备份入口。程序升级后首次启动即清空，用户须在升级前自行备份数据库文件 |

### 非违规但需说明的设计选择

| 选择 | 理由 |
| --- | --- |
| 存 `name_folded` 而不是用 `COLLATE NOCASE` | SQLite 的 `NOCASE` 只折叠 ASCII，达不到「全部 Unicode」的要求 |
| 新增 `caseless` 依赖 | `str::to_lowercase` 不是完整 case folding（`ß` 不折叠为 `ss`），标准库没有等价能力 |
| 迁移用 `DROP` 重建而不是 `ALTER` + 清洗 | Q1 决策是清空全部数据，`ALTER` 路径的清洗逻辑不会被使用 |
| `scope` 改为完整句子而不是新增错误变体 | 一个变体表达两种作用域，避免接触面分裂（宪法 IV / VI） |
| 迁移不提供 `--dry-run` 或备份 | 没有当前消费者（宪法 VI）；规格 Assumptions 已声明用户自行备份 |

## Phase 完成情况

- **Phase 0**: [research.md](./research.md) —— 4 项决策，无遗留 NEEDS CLARIFICATION
- **Phase 1**: [data-model.md](./data-model.md)、[contracts/domain-api.md](./contracts/domain-api.md)、[quickstart.md](./quickstart.md)
- **Phase 2**: 待 `/speckit.tasks` 生成任务清单
