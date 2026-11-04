# AGENTS.md

Rust + spec-kit 的 vibe coding 模板。本文件是给编码代理的速查入口，**只做索引，不定义规则**：
细则一律以 [`.specify/memory/constitution.md`](.specify/memory/constitution.md) 为准，
冲突时以宪法为准。

## 目录地图

| 路径 | 放什么 |
| --- | --- |
| `src/` | 源码，模块按职责命名；测试镜像到 `tests/<module>_test.rs` |
| `src/app/` | 界面层：`app.rs` 状态与事件循环、`main_view` 结构树、`today_view` 今日清单、`rename_dialog` / `delete_dialog` 弹窗、`tests.rs` headless 验收 |
| `.specify/memory/constitution.md` | 八条原则与治理规则，唯一权威 |
| `.specify/templates/` | spec / plan / tasks / checklist 模板 |
| `.specify/scripts/` | spec-kit 辅助脚本（PowerShell / Python） |
| `.codebuddy/commands/` | `/speckit.*` 命令定义 |
| `specs/` | 每个 feature 的 spec、plan、tasks（由 `/speckit.specify` 创建） |
| `.github/workflows/ci.yml` | 六个 CI 门禁 |
| `.github/CODEOWNERS` | 核心模块与所有者 |

## 命令

```bash
cargo run
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo machete     # 未使用依赖，需 cargo install cargo-machete --locked
cargo audit       # 依赖漏洞
```

本地把前四条跑一遍，就等于复现了 CI 的格式、lint 与测试门禁。

## 硬约束（判据以宪法对应小节为准）

| 约束 | 一句话判据 | 详见 |
| --- | --- | --- |
| 文件规模 | 超过上限先拆再改，不堆着写 | I |
| 质量与可维护性 | 无死代码、无注释代码、TODO 必须带 issue | II |
| 测试先行 | 先写测试并看到它失败，再写最少实现 | III |
| 对外接触面 | CLI 与库 API 命名统一，错误必须可执行 | IV |
| 性能 | 涉及性能的变更必须附前后测量 | V |
| 简单性 | 没有当前消费者就不写配置、开关与抽象 | VI |
| 可观测性 | 结构化字段，错误只记一次，stdout 放数据 / stderr 放日志 | VII |
| 安全 | 输入在边界校验，禁止拼接注入，机密不入库 | VIII |

## 提交策略

- **自动提交**：每完成一个 spec-kit 阶段（`/speckit.specify`、`/speckit.plan`、
  `/speckit.tasks`，以及 `/speckit.implement` 的每个任务批次），立即在功能分支上提交产物，
  不再逐次征求确认。
- 提交按逻辑拆分：规格与设计产物一个提交，流程或治理调整另一个提交；提交信息用
  `<type>(<scope>): <说明>` 形式。
- 只在两种情况下停下来询问：当前不在功能分支上，或需要改动 `main`。

## 禁止事项

- 不手改 `Cargo.lock`，用 `cargo` 命令更新。
- 不改 `.specify/memory/constitution.md`，除非用户明确要求；改了按治理条款递增版本。
- 不为让 CI 变绿而添加 lint 抑制。死代码豁免只接受
  `#[expect(dead_code, reason = "...，见 #<issue>")]`。
- 不提交 `.env`、令牌或私钥。
- 不直接提交到 `main`：变更走 PR，分支保护开启后会被拦下。

## 冲突时

宪法 > 本文件 > 其他惯例。规格与宪法冲突时，先改宪法再改规格。
