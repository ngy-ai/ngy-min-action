# AGENTS.md

本文件约束在本仓库工作的 AI 代理与协作者。规则优先于个人习惯；与用户当次明确指令冲突时，
以用户指令为准，并在提交说明里注明例外。

## 项目速览

- Rust 2024（版本见 `rust-toolchain.toml`），桌面应用，界面用 gpui / gpui-kit。
- `src/app/`：界面层（根视图、结构树、三个页签、各类对话框）；`src/model*`、`src/store*`、
  `src/schedule*`、`src/review*` 是领域与数据层，尽量不依赖 gpui；`tests/` 是无头验收。
- 运行数据在 `%LOCALAPPDATA%\ngy-min-action`（`data.db`、`app.lock`），路径解析见 `src/config.rs`。

## 常用命令

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run
```

在 Windows 上跑构建/测试前先停掉正在运行的实例，否则 exe 被占用。

## 规则

### 1. 自动提交

每个阶段的产物完成后立即在当前功能分支提交，不逐次征求确认：规格/设计产物一个提交、
代码改动按逻辑另一个提交、流程或治理调整再一个提交。提交信息用 `<type>(<scope>): <说明>`
（`feat` / `fix` / `refactor` / `docs` / `test` / `build` / `chore`），说明写清做了什么，
不写「update files」这类空话。

只在两种情况下停下来问：当前不在功能分支上，或改动需要落到 `main`。

### 2. 文件规模：单文件不超过 700 行

超过 700 行先拆再改，不堆着写。按职责拆分（一个视图/一个对话框/一组数据操作一个模块），
拆分单独成一次提交，改动再单独提交。新增文件时就按这个上限规划，不要「先写完再拆」。

### 3. 质量与可维护性

- 无死代码：没人调用的函数、字段、导出要么删掉，要么按规则 7 的方式显式豁免。
- 无注释掉的代码：历史由 git 保存，不留在源码里。
- TODO 必须带 issue：`// TODO(#123): …`。没有 issue 就不写 TODO —— 要么当场做完，
  要么先开 issue 再写。

### 4. 测试先行

先写测试，运行它并看到它失败，再写最少实现让它通过。测试放 `tests/` 或同文件的
`#[cfg(test)] mod`。改动行为必须同步补/改测试；不许为了变绿而删断言或放宽断言。
提交前 `cargo test` 必须全绿。

### 5. 对外接触面：命名统一、错误可执行

- CLI 参数、子命令与库 API 共用同一套领域词（goal / category / task / action），
  不出现同义两套命名（如 `item` 与 `task` 混用）。
- 错误必须可执行：说清发生了什么 + 下一步做什么，例如
  `Err("请先创建目标，再添加分类")`。禁止裸 `Err("失败")`、禁止把内部错误码直接甩给用户。

### 6. 不手改 `Cargo.lock`

依赖增删与升级用 `cargo add` / `cargo remove` / `cargo update`，让 `Cargo.lock` 由 cargo 生成。
手改 `Cargo.toml` 后至少跑一次 `cargo build` 让 lock 同步，并与清单改动一起提交。

### 7. 不为让 CI 变绿而抑制 lint

不用 `#[allow(...)]`、`#[cfg_attr(..., allow(...))]`、`#![allow(...)]` 等手段消掉告警。
告警要么修代码解决，要么（仅限死代码）写成
`#[expect(dead_code, reason = "…原因…，见 #<issue>")]`：必须写清为什么暂时保留并附 issue 号，
条件不满足就直接删代码。

### 8. 不提交敏感信息

不提交 `.env`、令牌、私钥、证书、带个人路径的日志。`.gitignore` 已忽略 `/logs`、
`github_token.txt`、`/TODO.md`，不要用 `git add -f` 绕过。一旦误提交，立即报告并轮换密钥。

### 9. 不直接提交到 `main`

变更只在自己的功能分支上做（如 `016-structure-tree-ui`），经 PR 合入 `main`；分支保护开启后
直推会被拦下。不 `push --force` 到 `main`/`master`，破坏性操作（`reset --hard`、强制推送）
需用户明确要求。
