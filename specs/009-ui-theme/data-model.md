# Data Model: 界面美化与明暗主题

**Feature**: `009-ui-theme` | **Date**: 2026-11-05

## 实体

### ThemeChoice（领域枚举）

用户在界面上的主题选择，定义于 `src/model/theme.rs`，由 `src/model.rs` 通过 `pub use` 转出。

| 取值 | 持久化字符串 | 含义 |
| --- | --- | --- |
| `System` | `system` | 跟随操作系统明暗；默认值 |
| `Light` | `light` | 固定浅色 |
| `Dark` | `dark` | 固定深色 |

- **转换**：`as_str()` 给出持久化字符串；`parse(raw: &str) -> ThemeChoice` 做边界校验，任何未知值返回 `System`
- **映射**：界面层把 `ThemeChoice` 映射为 egui 的 `ThemePreference::{System, Light, Dark}`；领域层不依赖 GUI 库
- **不变量**：`parse(choice.as_str()) == choice` 对三个取值都成立

### 设置键 `theme`

复用 `settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)`，新增一行 `key = 'theme'`。

| 属性 | 规则 |
| --- | --- |
| 缺省（无此行） | 读取返回 `System`，不自动写库 |
| 非法值 / 空串 | 读取返回 `System`，不自动覆盖原值；记录一条警告 |
| 写入 | `INSERT ... ON CONFLICT(key) DO UPDATE`，参数绑定，只允许三个合法字符串 |
| 与其他设置的关系 | 与 `daily_cap` 互不影响，写入 `theme` 不改变任何其它行 |

## 状态转换

```text
首次启动（无记录） ──> System
System ──用户选浅色──> Light ──持久化──> 重启后 Light
System ──用户选深色──> Dark  ──持久化──> 重启后 Dark
Light / Dark ──用户选跟随系统──> System ──持久化──> 重启后 System
System 且系统主题变化 ──> 界面跟随，偏好值仍为 System（不写库）
未知持久化值 ──> 读取回退 System（不写库）
```

## 不受影响的数据

`goals`、`categories`、`tasks`、`actions`、`review_progress`、`reminder_state` 与 `settings.daily_cap` 全部不读写、不迁移、不删改。主题偏好不参与调度、统计或提醒逻辑。

## 校验规则

- 存储层读取：字符串先 trim 再匹配；不匹配按 `System` 处理
- 存储层写入：只接受 `ThemeChoice` 枚举值，不接受任意字符串，因此无需额外的注入防护
- 错误模式：`DomainError::Store` 只在 SQLite 读写失败时出现；`set_theme_choice` 无 `InvalidValue` 分支（枚举已保证取值合法），契约注释中写明
