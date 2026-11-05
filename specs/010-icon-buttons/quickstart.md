# Quickstart 验证指南: 用图标替换操作按钮

**Feature**: `010-icon-buttons` | **Date**: 2026-11-05

验证清单，不是实现说明。逐按钮映射见 [contracts/icon-buttons.md](./contracts/icon-buttons.md)。

## 前置条件

- Rust 工具链由 `rust-toolchain.toml` 固定（1.97）
- Windows 10/11 x64 桌面；沿用 009 的浅色／深色主题
- 用隔离的 `%LOCALAPPDATA%` 启动 GUI，避免影响真实数据

```powershell
$root = 'e:\projects\ngy-min-action\target\gui-010'
New-Item -ItemType Directory -Force "$root\appdata\ngy-min-action" | Out-Null
$env:LOCALAPPDATA = "$root\appdata"
cargo build --release
Start-Process 'target\release\ngy-min-action.exe'
```

用 `settings.theme` 的 `light` / `dark` 两个取值各重启一次，得到两套截图。

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
| `src/app/tests/icons.rs` | 图标映射表每个条目形状与提示文案非空；危险语义只用于危险操作 |
| `src/app/tests/icons.rs` | 两套主题下含图标的主界面、今日清单与全部弹窗渲染一帧不 panic |
| `src/app/tests/icons.rs` | 点击图标后业务状态与替换前一致（完成／跳过／改名入口／删除入口） |
| 既有 headless 验收 | 所有原先依赖按钮文字的验收用例仍通过 |

## 手工验收场景

### 场景 1：图标替换（C-01、C-02、C-06）

1. 在浅色与深色下各查看主界面、任务行、今日清单与六个弹窗
2. 逐个悬停图标，核对提示文案

**预期**：范围内不再有中文文字按钮；每个图标都有与原文一致的提示；主题下拉、表单标签、统计数值、空状态与错误提示仍是文字。

### 场景 2：行为不变（C-01、C-03）

1. 点击完成、跳过、撤销、改名、暂停／继续、重置复习、删除、保存、取消、关闭、统计
2. 特别检查危险操作是否仍弹出二次确认

**预期**：所有操作结果与替换前一致；草稿不丢失；危险操作颜色与普通操作可区分。

### 场景 3：布局与主题（C-04、C-05）

1. 把窗口拉窄，检查任务行与弹窗操作区
2. 切换主题后检查已打开的弹窗

**预期**：图标不重叠、不溢出，命中区域易点击；切换主题后图标同步换色，无豆腐块。

### 场景 4：约束复核（C-07）

```bash
git grep -nE 'Color32::' -- src
git diff main -- Cargo.toml
```

**预期**：`Color32::` 只出现在 `src/app/theme.rs`；`Cargo.toml` 无变化。

## 通过标准

- 六项 CI 门禁全部通过
- 场景 1~3 的两套截图均无缺字、溢出或可读性退化
- 悬停提示逐项与契约表一致
- 宪法 I 的文件规模上限未被突破
