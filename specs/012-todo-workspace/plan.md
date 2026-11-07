# Implementation Plan: 待办工作区

**Branch**: `012-todo-workspace` | **Date**: 2026-11-07 | **Spec**: [spec.md](./spec.md)

## Summary

在现有行动模型上做三件事：给待办补文字备注与图片附件；把主界面扩成「菜单栏 + 工具条 + 今日／历史页签 + 状态栏」；把数据位置从写死改为启动配置，并提供「目录选择 → 复制数据库与附件 → 校验 → 切换 → 清理旧位置」的立即迁移。

## Phase 0 / Phase 1 结论

| 决策 | 结论 |
| --- | --- |
| 待办模型 | 沿用 `actions` 与 `UNIQUE(task_id, on_date)`；新增 `note TEXT` 列，不改生成、上限、顺延与复习规则 |
| 图片记录 | 新表 `action_images(id, action_id, file_name, original_name, byte_size, created_at)`；文件放 `<数据目录>/attachments/<action_id>/<file_name>` |
| 图片校验 | 扩展名白名单 + `image` 解码探测真实格式；PNG/JPEG/WebP/BMP，单张 ≤ 10 MiB，拒绝损坏文件；加入时复制，失败不留孤儿文件 |
| 图片展示 | `egui_extras` 的 `image` 加载器；`file://` URI + `max_size` 缩略图；文件缺失时显示占位与文件名 |
| 历史查询 | `actions_before(date, limit)` 游标分页，按 `on_date DESC, seq, id` 排序；页大小 50；不一次性加载全表（宪法 V） |
| 菜单栏 | `egui::MenuBar`：「文件 → 存储位置…」「视图 → 今日／历史」「帮助 → 关于」；主题入口保留在工具条，避免重复 |
| 状态栏 | `Panel::bottom`：数据位置（中间省略 + 悬停完整）、今日待办数、版本 |
| 页签 | 中央区 `TodoTab::{Today, History}`；切换不重置历史游标与滚动位置 |
| 位置配置 | 启动目录（`%LOCALAPPDATA%\ngy-min-action`）下的 `location` 文本文件，内容为绝对数据目录；缺失或非法回退默认目录并 warn |
| 单实例锁 | `app.lock` 移到启动目录，与数据位置解耦 |
| 迁移 | `rusqlite` 的 `backup` 特性把在线库复制到目标；递归复制 `attachments/`；新库 `integrity_check` 通过后原子写配置；替换 Store、重载、再清理旧目录（失败只 warn） |
| 迁移拒绝条件 | 目标与源相同、目标在源内（或源在目标内）、目标已有 `data.db`、目录不可写 |
| 依赖 | 新增 `rfd`（目录／文件选择）、`egui_extras`（image）、`image`（解码校验）；`rusqlite` 增加 `backup` feature；不引入网络能力 |
| schema | `SCHEMA_VERSION = 3`；2 → 3 只执行 `ALTER TABLE actions ADD COLUMN note`、建 `action_images` 与索引；1 → 3 维持原有「清空重建」策略 |
| 日志 | 新增操作一律 `op/result/elapsed_ms`；迁移记录源、目标、字节数；错误只在界面或入口记录一次 |

## Technical Context

**Language/Version**: Rust 1.97（edition 2024）

**Primary Dependencies**: `eframe` / `egui` 0.37、`rusqlite` 0.41（bundled + backup）、`rfd` 0.17、`egui_extras` 0.37（image）、`image` 0.25（仅 png/jpeg/webp/bmp）

**Storage**: SQLite schema 3；附件目录 `<数据目录>/attachments/`；启动配置 `<启动目录>/location`

**Testing**: `cargo test --all-targets --all-features`；新增 `tests/note_test.rs`、`tests/storage_test.rs` 与 `src/app/tests/{workspace,history}.rs`；GUI 截图人工确认

**Target Platform**: Windows 10/11 x64 桌面

**Project Type**: desktop-app（单 crate，lib + bin）

**Performance Goals**: 沿用宪法 V 桌面预算；历史页首屏 ≤ 50 行；图片只在打开备注或滚动到可视区时解码，缩略图上限 96×96；迁移按文件流式复制，不整目录读入内存

**Constraints**: 单文件 600 行预警线；无 lint 抑制；颜色只取语义令牌；完全离线；不改变调度与复习规则

## Constitution Check

| 原则 | 判据 | 结论 |
| --- | --- | --- |
| I. 文件规模 | 新增模块按职责拆分（`store/note.rs`、`attachment.rs`、`config.rs`、`storage.rs`、`app/menu_bar.rs`、`app/status_bar.rs`、`app/history_view.rs`、`app/note_dialog.rs`、`app/storage_dialog.rs`）；`app.rs` 只加装配 | PASS |
| II. 质量与可维护性 | 无死代码、无裸 TODO；公共接口带契约文档；错误文案可执行 | PASS |
| III. 测试先行 | 备注、图片校验、迁移、历史分页、配置文件、UI 渲染均先写失败用例 | PASS |
| IV. 接触面一致性 | 不新增 CLI；新增存储方法与配置错误类型沿用 `DomainError` / `StoreError` 风格 | PASS |
| V. 性能与效率 | 历史分页 + 图片缩略图上限；迁移流式复制；无热路径变更 | PASS |
| VI. 简单性 | 复用 `actions` 与 `settings`；不建仓库层、不做图片编辑；只加当前功能需要的配置项 | PASS |
| VII. 可观测性 | 迁移、备注保存、图片加入/移除、历史加载均结构化打点；错误只记一次 | PASS |
| VIII. 安全 | 图片路径由程序生成，原始文件名只做展示；扩展名 + 解码双重校验；不拼接 SQL；目录校验绝对路径与可写性 | PASS |

## Project Structure

```text
specs/012-todo-workspace/
├── spec.md
├── checklists/requirements.md
├── plan.md
└── tasks.md

src/
├── config.rs                 # 启动目录、location 文件读写
├── storage.rs                # 迁移编排：复制、校验、写配置、清理
├── attachment.rs             # 图片校验、复制、删除、目录布局
├── run.rs                    # 启动目录 / 数据目录 / 锁位置 / 图片加载器
├── store/
│   ├── schema.rs             # schema 3 与增量迁移
│   ├── note.rs               # 待办备注与图片行
│   └── action.rs             # 历史游标查询
└── app/
    ├── menu_bar.rs           # 顶部菜单栏
    ├── status_bar.rs         # 底部状态栏（原 status_bar 改名 toolbar.rs）
    ├── toolbar.rs            # 原顶栏：日期、上限、主题、统计
    ├── history_view.rs       # 历史页签
    ├── note_dialog.rs        # 备注弹窗：文字、图片、拖拽
    ├── storage_dialog.rs     # 存储位置设置与迁移
    └── tests/
        ├── workspace.rs      # 菜单、状态栏、页签渲染
        └── history.rs        # 历史分页与备注展示
```

## Complexity Tracking

- 新增三个顶层模块（`config`、`storage`、`attachment`）是为了把「启动位置」「文件搬迁」「图片文件」从 `run`、`store`、`app` 中隔离；均在宪法 VI 允许的单 crate 内。
- 新增三项依赖均有当前消费者：`rfd` 用于目录／文件选择，`egui_extras` + `image` 用于缩略图与格式校验；不启用网络 feature。
- 未引入图片编辑、Markdown、云同步、历史删除等无当前消费者的能力。
- 设计文档不单独生成 `research.md` / `data-model.md` / `contracts/` / `quickstart.md`，结论并入本文件与 `tasks.md` 的验收记录。
