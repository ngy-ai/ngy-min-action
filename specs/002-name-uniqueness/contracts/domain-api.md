# Phase 1 Contract: 名称校验相关库 API

**Feature**: `002-name-uniqueness` | **Date**: 2026-11-03

**基线**: 增量修订 `specs/001-min-action-core/contracts/domain-api.md` 中的四个方法，其余签名与语义不变。

## 公共辅助

```rust
/// 校验并规范化名称：去除两端空白、拒绝空值与控制字符、限制长度。
///
/// 错误：`DomainError::InvalidName`（`field` 指出是哪一项输入）。
pub fn validate_name(raw: &str, field: &'static str, max: usize) -> Result<String, DomainError>;

/// 计算名称的比较键：Unicode 默认大小写折叠。
///
/// 输入必须已经通过 `validate_name`（已去除两端空白）。
pub fn fold_name(name: &str) -> String;
```

**契约**: `fold_name` 是纯函数，同一输入恒返回同一输出；对已折叠结果再次折叠结果不变（幂等）。

## Store

### create_goal

```rust
pub fn create_goal(&self, name: &str, description: Option<&str>) -> Result<GoalId, DomainError>;
```

- **前置**: `name` 去两端空白后 1..=80 字符，无控制字符
- **成功**: 写入 `name`（trim 后原样，保留内部空白）与 `name_folded`，返回新 ID
- **错误**:
  - `DomainError::InvalidName { field: "目标名称", .. }` —— 名称为空、超长或含控制字符
  - `DomainError::DuplicateName { scope: "目标名称必须唯一", name }` —— 折叠后与任一已有目标（含已删除）相同
  - `DomainError::Store` —— 读写失败
- **副作用**: 失败时不写入任何行

### rename_goal

```rust
pub fn rename_goal(&self, id: GoalId, name: &str) -> Result<(), DomainError>;
```

- **前置**: 同上
- **成功**: 同时更新 `name` 与 `name_folded`
- **错误**:
  - `DomainError::NotFound { entity: "目标", id }` —— 目标不存在或已删除
  - `DomainError::DuplicateName { scope: "目标名称必须唯一", name }` —— 与其他目标冲突（与自身同名不算冲突）
  - `DomainError::InvalidName`、`DomainError::Store` 同 `create_goal`
- **副作用**: 失败时名称与折叠列均不变

### create_category

```rust
pub fn create_category(&self, goal: GoalId, name: &str) -> Result<CategoryId, DomainError>;
```

- **前置**: `name` 去两端空白后 1..=80 字符，无控制字符；`goal` 存在且未删除
- **成功**: 写入两列，返回新 ID
- **错误**:
  - `DomainError::NotFound { entity: "目标", id }` —— 目标不存在或已删除
  - `DomainError::DuplicateName { scope: "同一目标下的分类名称必须唯一", name }` —— 同一目标下折叠后相同（含已删除分类）；不同目标之间不冲突
  - `DomainError::InvalidName`、`DomainError::Store`
- **副作用**: 失败时不写入任何行

### rename_category

```rust
pub fn rename_category(&self, id: CategoryId, name: &str) -> Result<(), DomainError>;
```

- **前置**: 同上
- **成功**: 同时更新两列
- **错误**:
  - `DomainError::NotFound { entity: "分类", id }` —— 分类不存在或已删除
  - `DomainError::DuplicateName { scope: "同一目标下的分类名称必须唯一", name }` —— 与本目标下其他分类冲突
  - `DomainError::InvalidName`、`DomainError::Store`
- **副作用**: 失败时两列均不变

## 错误文案

`DomainError::DuplicateName` 的展示模板调整为：

```text
{scope}：已存在名称「{name}」，请换一个名称或先删除原有的那一项
```

| 场景 | `scope` 取值 | 渲染结果 |
| --- | --- | --- |
| 目标重名 | `目标名称必须唯一` | 目标名称必须唯一：已存在名称「学习英语」，请换一个名称或先删除原有的那一项 |
| 分类重名 | `同一目标下的分类名称必须唯一` | 同一目标下的分类名称必须唯一：已存在名称「口语」，请换一个名称或先删除原有的那一项 |

**兼容性**: 001 的文案为「{scope} 下已存在名称「{name}」，请换一个名称或先删除原有的那一项」。此处改为冒号连接，属于接触面文字变更，行为（拒绝 + 不落行）不变。既有断言该文案的测试需同步更新。

## 迁移相关

```rust
/// 打开或创建数据库，并完成建表与升级。
///
/// 错误：`StoreError::Open`（路径不可写）、`StoreError::Migrate`（升级失败，已回滚）。
pub fn open(path: &Path) -> Result<Self, StoreError>;
```

**契约**: 从 `user_version < 2` 的库升级时，全部业务数据被清空且不可恢复；升级在单个事务内完成，失败时不改变原库内容。
