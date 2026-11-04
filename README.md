# ngy-min-action

用自然语言驱动 AI 开发 Rust 项目的骨架：工具链、格式化、lint、测试与治理门禁已预置，
开发规范以 [`.specify/memory/constitution.md`](.specify/memory/constitution.md) 为准。

## 环境要求

- [rustup](https://rustup.rs/)；Rust 版本由 `rust-toolchain.toml` 固定为 1.97，
  rustfmt 与 clippy 随工具链安装
- 本地想跑依赖检查时装 `cargo install cargo-machete --locked`（CI 会自动装）

## 快速开始

```bash
cargo run
```

启动后是一个 Windows 桌面常驻程序：

- 关闭主窗口即退出程序；最小化则继续驻留，跨日检测与每日提示不受影响
- 同一台机器只允许一个实例：重复启动会直接退出并提示「程序已在运行」
- 数据保存在 `%LOCALAPPDATA%\ngy-min-action\data.db`（SQLite），日志与实例锁在同一目录
- 程序运行期间，每个本地日会弹出一次置顶提示窗口，关闭即视为当天已提示

### 它做什么

1. 建立「目标 → 分类 → 任务」三层结构，例如「学习英语 → 口语 → 每周学习一个连读」
2. 目标、分类、任务都能改名；删除任务前先说明会作废多少未完成行动，确认后这些行动转为「已过期」
3. 每天自动生成一份最小行动清单，受全局上限（默认 5 条）与每任务上限（每天 1 条）双重约束，超出的顺延
4. 完成行动后按遗忘曲线 `[1, 2, 4, 7, 15, 30]` 天安排复习；跳过不推进，撤销可回退
5. 常驻后台，每天弹一次置顶窗口提示当天未完成数量

## 目录结构

| 路径 | 作用 |
| --- | --- |
| `src/` | 源码；模块按职责命名，测试镜像源码布局 |
| `.specify/memory/constitution.md` | 项目宪法，八条原则的唯一定义处 |
| `AGENTS.md` | 给编码代理的速查入口（索引，不含细则） |
| `.specify/templates/` | spec-kit 的 spec / plan / tasks / checklist 模板 |
| `.specify/scripts/` | spec-kit 的 PowerShell 与 Python 辅助脚本 |
| `.codebuddy/commands/` | `/speckit.*` 命令定义 |
| `.github/workflows/ci.yml` | CI：格式、lint + 依赖、测试、宪法治理扫描 |
| `rust-toolchain.toml` | 固定工具链与组件 |
| `rustfmt.toml` | `edition` / `style_edition` = 2024 |
| `Cargo.toml` | 包信息与 MSRV（`rust-version`） |

## 质量门禁

与 `ci.yml` 的 job 一一对应：

| 检查 | 对应原则 |
| --- | --- |
| `cargo fmt --check` | II |
| `cargo clippy --all-targets --all-features -- -D warnings` | II |
| `cargo machete`（未使用依赖） | II |
| `cargo test --all-targets --all-features` | III |
| `cargo audit`（依赖漏洞） | VIII |
| 机密扫描（私钥块、云密钥、平台令牌、误提交的 `.env`） | VIII |
| TODO 归属扫描 + `#[allow(dead_code)]` 拦截 | II |

本地一次跑完：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo machete
```

## 项目规范

宪法八条原则：

| 编号 | 原则 | 要点 |
| --- | --- | --- |
| I | 源码文件规模与拆分 | 单文件 700 行硬上限、600 行预警线，先拆再改 |
| II | 代码质量与可维护性 | 公共接口写契约、不留死代码、TODO 必须带 issue |
| III | 测试先行开发 | 先写测试并看到它失败，再用最少实现转绿 |
| IV | 对外接触面一致性 | CLI / 库 API / 错误消息统一，错误必须可执行 |
| V | 性能与效率 | 默认预算（p95 门禁 / p99 观察）、禁止二次复杂度与逐项 IO、变更须附测量、回归 >10% 需理由 |
| VI | 简单性与 YAGNI | 默认单 crate、不预留配置与推测性抽象、三行重复优于过早抽象 |
| VII | 可观测性与结构化日志 | 结构化字段、错误只记一次、stdout 放数据 / stderr 放日志、不记敏感信息 |
| VIII | 安全基线 | 边界校验输入、禁止拼接注入、机密不入库、依赖扫漏洞 |

规则细节只在宪法里维护，本文件不复述，避免两处漂移。

## spec-kit 工作流

典型顺序：

1. `/speckit.constitution` —— 修订治理原则（可选）
2. `/speckit.specify` —— 写清要做什么，产出 `specs/<feature>/spec.md`
3. `/speckit.clarify` —— 消除规格里的歧义
4. `/speckit.plan` —— 技术方案，产出 `plan.md`，含 Constitution Check
5. `/speckit.tasks` —— 拆成可执行任务
6. `/speckit.implement` —— 执行

辅助命令：`/speckit.analyze`、`/speckit.checklist`、`/speckit.taskstoissues`、
`/speckit.converge`。

## 作为模板使用

1. 改 `Cargo.toml` 的 `name`，crate 名会自动变成下划线形式
2. 更新 `.github/CODEOWNERS`，把所有者换成你自己或团队
3. 第一次 `/speckit.plan` 时确认宪法 V 的默认预算是否沿用，未确认即视为沿用
4. 按需调整 `ci.yml` 的工具链版本与 job
5. 配置分支保护（见下），否则「变更必须走 PR」只是口头约定
6. 重写本 README

### 分支保护（只能在 GitHub 上手动开启）

仓库里的任何文件都管不住这条，只有分支保护能让宪法「拉取请求」一节真正生效：

1. Settings → Branches → Add branch protection rule，目标分支 `main`；
2. 勾选 **Require a pull request before merging**。单开发者仓库可把批准数设为 0，
   由 Governance 里的单开发者条款以自我证明兜底；
3. 勾选 **Require status checks to pass**，并选中全部六个：
   `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、
   `Constitution governance scan`、`cargo audit`、`Secret scan`；
4. 勾选 **Do not allow bypassing the above settings**，否则管理员仍能直接推送。

开启之前，直接 `git push origin main` 是会成功的——本模板自身的提交历史就是例子。

## 容易踩的约定

- 测试镜像源码：`src/<module>/` 对应 `tests/<module>_test.rs`
- TODO 只能写成 `TODO(#<issue>): 说明`；裸 `TODO:`、`TODO(TBD)` 会被 CI 拦下
- 死代码豁免只接受 `#[expect(dead_code, reason = "<理由>，见 #<issue>")]`，
  `#[allow(dead_code)]` 会被 CI 拦下
- 未使用的依赖会让 `cargo machete` 失败，先删依赖再加代码，不要反过来
- 基准工具（`criterion`、`hyperfine`）只在真正要测量时引入，且与第一个基准同时提交
