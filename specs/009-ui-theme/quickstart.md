# Quickstart 验证指南: 界面美化与明暗主题

**Feature**: `009-ui-theme` | **Date**: 2026-11-05

本文件是验证清单，不是实现说明。契约见 [contracts/ui-theme.md](./contracts/ui-theme.md)，偏好实体见 [data-model.md](./data-model.md)。

## 前置条件

- Rust 工具链由 `rust-toolchain.toml` 固定（1.97）
- Windows 10/11 x64 桌面，可在系统设置里切换浅色/深色
- 用隔离的 `%LOCALAPPDATA%` 启动 GUI，避免影响真实数据

```powershell
$root = 'e:\projects\ngy-min-action\target\gui-009'
New-Item -ItemType Directory -Force "$root\appdata\ngy-min-action" | Out-Null
$env:LOCALAPPDATA = "$root\appdata"
cargo build --release
Start-Process 'target\release\ngy-min-action.exe'
```

## 自动化验证

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo machete
cargo audit
```

| 用例 | 覆盖 |
| --- | --- |
| `tests/theme_test.rs` | 默认「跟随系统」、三种取值往返、未知值回退、写入不影响 `daily_cap` 与业务表 |
| `src/app/tests/theme.rs` | 切换后主题状态、弹窗随主题渲染、业务快照不变、系统主题注入时跟随 |
| `src/app/tests/theme.rs` | 两套令牌的正文/弱化/警告/危险色对比度达标 |

## 手工验收场景

### 场景 1：切换主题（C-01）

1. 启动程序，打开「统计」面板
2. 顶栏依次选择浅色、深色

**预期**：两次切换后主界面、结构树、今日清单、统计面板与所有弹窗整体换色，无新旧配色混杂；统计面板不关闭，草稿类弹窗内容不丢失。

### 场景 2：重启恢复（C-03 / C-04）

1. 选择深色，关闭程序再启动
2. 选择浅色，重启；再选择「跟随系统」，重启

**预期**：三次重启首帧都是所选主题，没有先亮后暗（或相反）的闪烁；首次启动无偏好时按系统主题显示。

### 场景 3：跟随系统（C-02 / C-05）

1. 选择「跟随系统」，在 Windows 设置里切换系统明暗
2. 手工把 `settings` 表的 `theme` 值改成无法识别的字符串后重启

**预期**：步骤 1 界面在 2 秒内跟随且不需要重启；步骤 2 正常启动并按系统主题显示，不报错。

### 场景 4：视觉层级（C-06 / C-07）

1. 在浅色与深色下各截一张主界面、任务行、空状态、错误提示、五个弹窗
2. 检查主要/次要/危险操作是否可区分，列是否对齐，长名称截断是否一致
3. 运行 `git grep -nE 'Color32::' -- src` 检查颜色字面量

**预期**：两套主题下文字都清晰可读；危险操作仍需二次确认；`Color32::` 只出现在令牌定义文件；主题切换前后行动、统计、每日上限完全一致。

## 通过标准

- 六项 CI 门禁全部通过
- 场景 1~3 的可观察结果与预期一致
- 场景 4 的两套截图均无明显可读性或对齐问题，颜色字面量检查通过
- 宪法 V 的桌面预算未被突破（本次无热路径变更，可用 `pwsh scripts/perf.ps1` 复核）
