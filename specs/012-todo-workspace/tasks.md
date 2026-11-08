# Tasks: 待办工作区

**Input**: `specs/012-todo-workspace/` 的 spec、plan

**Tests**: 宪法 III 要求测试先行；每个阶段先写并观察到失败，再写最少实现

**Organization**: 先数据与迁移地基，再按用户故事接入界面

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、不依赖未完成任务）
- **[Story]**: 所属用户故事（US1 / US2 / US3 / US4）

---

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 核对分支为 `012-todo-workspace`，确认 `Cargo.toml` 仅新增 `rfd`、`egui_extras`（image）、`image`（png/jpeg/webp/bmp）与 `rusqlite` 的 `backup` feature
- [x] T002 先跑一次 `cargo fmt --check` / `clippy` / `test` 记录基线，确保后续失败只来自本 feature

---

## Phase 2: Foundational (Blocking Prerequisites)

- [x] T003 [P] 在 `tests/note_test.rs` 写失败用例：schema 2 库升级到 3 后既有行动保留、`note` 默认空；备注设置/清空/超长拒绝；图片行增删与按待办查询；历史游标分页无重复
- [x] T004 实现 `src/store/schema.rs` 的 `SCHEMA_VERSION = 3` 与 2→3 增量迁移、`actions.note` 列、`action_images` 表与索引；新增 `src/store/note.rs` 与 `actions_before`，让 T003 转绿
- [x] T005 [P] 在 `tests/storage_test.rs` 写失败用例：`location` 缺失/非法回退默认、绝对路径往返、迁移拒绝同目录/目标已有数据/不可写、成功后附件完整、失败保留原位置
- [x] T006 实现 `src/config.rs`（启动目录、`location` 原子读写）与 `src/attachment.rs`（格式/大小校验、复制、删除、缺失占位）
- [x] T007 实现 `src/storage.rs` 迁移编排：`Store::backup_to`、附件递归复制、`integrity_check`、写配置、清理旧目录；`src/run.rs` 改用启动目录锁与可配置数据目录

---

## Phase 3: User Story 1 - 当日待办与图文备注 (Priority: P1)

- [x] T008 [US1] 在 `src/app/tests/note.rs` 写失败用例：备注保存/清空走同一入口、图片加入后数据目录出现副本、移除后文件消失、空备注不写行
- [x] T009 [US1] 实现 `src/app/note_dialog.rs`：文字上限、保存、文件选择、拖拽加入、缩略图、逐张移除；在 `src/app/today_view.rs` 加备注入口并显示备注/图片标记
- [x] T010 [US1] 在 `src/run.rs` 安装 `egui_extras` 图片加载器；缺失文件与损坏文件走占位路径，不 panic

---

## Phase 4: User Story 2 - 历史待办查看与管理 (Priority: P1)

- [x] T011 [US2] 在 `src/app/tests/history.rs` 写失败用例：历史按日期倒序、页大小固定、翻页无重复、空历史文案、备注只读展示
- [x] T012 [US2] 实现 `src/app/history_view.rs` 与页签状态 `TodoTab`：分页游标、完成/跳过/撤销复用既有结算入口，切换页签不重置游标

---

## Phase 5: User Story 3 - 菜单栏与状态栏 (Priority: P2)

- [x] T013 [US3] 在 `src/app/tests/workspace.rs` 写失败用例：菜单三组齐全、视图项切换页签、状态栏含路径/计数/版本、长路径省略、双主题渲染
- [x] T014 [US3] 把现有 `src/app/status_bar.rs` 纯重命名拆分为 `toolbar.rs`（独立提交，不改行为）
- [x] T015 [US3] 实现 `src/app/menu_bar.rs` 与新的 `src/app/status_bar.rs`，在 `src/app.rs` 装配菜单栏、工具条、页签与底栏

---

## Phase 6: User Story 4 - 存储位置设置与立即迁移 (Priority: P2)

- [x] T016 [US4] 在 `src/app/tests/storage_dialog.rs` 写失败用例：当前路径展示、选择目标后可迁移、迁移成功刷新路径、失败保留原路径并显示可执行错误
- [x] T017 [US4] 实现 `src/app/storage_dialog.rs`：`rfd` 目录选择、迁移中禁用、成功/失败状态；在「文件」菜单挂入口，状态栏同步新路径

---

## Phase 7: Polish & Cross-Cutting Concerns

- [x] T018 用隔离 `%LOCALAPPDATA%` 起 GUI：浅色/深色各截菜单栏、状态栏、今日与历史页签、备注弹窗；再迁移到临时目录，重启确认数据与图片来自新位置
- [x] T019 跑通 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo machete`、`cargo audit`；复核 `Color32::`、文件规模、lint 抑制与裸 TODO
- [x] T020 勾选本清单、填写验收记录，按逻辑拆分提交并写 PR 描述（含宪法条款复核与单开发者仓库自我证明）

---

## Dependencies

- T001 → T002 → T003 → T004
- T004 → T005 → T006 → T007
- T007 → T008 → T009 → T010
- T004 → T011 → T012
- T012 → T013 → T014 → T015
- T007 → T016 → T017
- T010 / T015 / T017 → T018 → T019 → T020

## Implementation Strategy

1. Phase 1-2：先打通数据、图片文件与迁移，界面之前就能用测试证明契约
2. Phase 3：今日待办备注与图片，完成后 US1 独立可用
3. Phase 4：历史分页与结算，复用既有 `settle_action` / `undo_last_settlement`
4. Phase 5：菜单栏、状态栏与页签外壳；`toolbar` 重命名单独提交
5. Phase 6：存储设置弹窗与迁移入口
6. Phase 7：截图、门禁、记录与 PR

## Notes

- 迁移成功后必须先替换 `Store` 再清理旧目录；旧连接未释放前不得删除源文件
- 图片加入顺序：先校验并复制到临时名，再写库；任一步失败都清理副本
- 历史页只做只读备注，结算按钮复用今日页的同一套方法，避免两套规则
- 菜单项用文字，不扩展 010 的图标映射；新增图标只在确有工具条按钮时加

---

## 验收记录（2026-11-08）

### 产物与范围

| 项 | 结果 |
| --- | --- |
| 分支 | `012-todo-workspace`，基线 `main` 的 `db92452`（011） |
| 提交 | `7afa40d` 规格与任务、`4594692` 数据与文件地基、`394b1a3` 界面外壳与图文备注、`a283d69` 图片加载修复 |
| 新增（源码） | `src/config.rs`、`src/attachment.rs`、`src/storage.rs`、`src/store/note.rs`、`src/app/{menu_bar,toolbar,status_bar,history_view,note_dialog,storage_dialog,describe}.rs` |
| 新增（测试） | `tests/note_test.rs`、`tests/storage_test.rs`、`src/app/tests/{workspace,history,note,storage}.rs` |
| 修改 | `src/app.rs`、`src/app/{today_view,icons}.rs`、`src/{model,store,run,lib,common}.rs`、`src/store/{action,schema}.rs`、`src/common/constants.rs`、既有测试与 bench 的 `Action { note }` 字段 |
| 依赖 | `Cargo.toml` 新增 `egui_extras`（`image`、`file`）、`image`（png/jpeg/webp/bmp）、`rfd`；`rusqlite` 加 `backup`。`Cargo.lock` 随之更新 |
| 数据 | schema 2 → 3 增量迁移：`actions.note`、`action_images` 表与索引；既有行动保留、`note` 默认空 |
| 设计文档 | 按 `plan.md` 裁剪，不生成 `research.md` / `data-model.md` / `contracts/` / `quickstart.md` |

### 测试先行

- T003/T005/T008/T011/T013/T016 均先写用例再写实现，失败信息保留在各阶段提交里。
- T018 截图阶段发现真实缺陷并沿同一流程修复：备注缩略图一律显示为加载失败占位。先加
  `note_image_uri_is_loadable_by_the_file_loader` 与 `image_uri_keeps_unc_paths`，在旧实现下
  观察到 `No matching BytesLoader`（`egui_extras` 只开了 `image`，`FileLoader` 未注册）；
  补 `file` feature 后观察到 `找不到网络路径（os error 53）`（`file://C:\...` 被当成 UNC 主机名）；
  改成 `file:///C:/dir/a.png` 与 `file://server/share/a.png` 两种形式后转绿。
- 当前规模：lib 62 条 + 集成 86 条 = 148 条，全部通过。

### GUI 截图（T018）

| 场景 | 文件（`target/gui-012/`，不入库） |
| --- | --- |
| 菜单栏 + 工具条 + 今日待办 + 状态栏（深/浅） | `dark-final.png`、`light-final.png` |
| 历史待办页签（深/浅） | `shot-history-dark.png`、`shot-history-light.png` |
| 备注弹窗（空备注、图片区提示） | `shot-note-dark.png` |
| 设置存储位置弹窗 | `shot-storage-dark.png` |
| 迁移成功（状态栏同步新路径） | `shot-migrate-dark.png` |
| 迁移后重启（数据与图片来自新位置） | `shot-restart-after-migrate.png`、`shot-note-image-fixed.png` |

- **验证工具偏差**：本机为 125% 缩放。旧截图脚本未声明 DPI 感知，`GetWindowRect` 返回 96 DPI
  虚拟化尺寸（894×598），而真实窗口是 1118×747，`PrintWindow` 结果底部约 45pt 被裁掉，一度
  表现为「状态栏不渲染」。用应用自身的 `ctx.request_screenshot` 取帧缓冲（1080×680）交叉验证后
  确认状态栏一直在正确位置；给脚本加 `SetProcessDpiAwareness(2)` 后截图恢复正常。该问题只影响
  验证手段，源码无改动。
- **输入注入偏差**：与 011 相同，本环境无法用合成鼠标事件驱动窗口。历史页签、备注弹窗与存储
  弹窗的截图由临时的 `NGY_PREVIEW` 入口在首帧前切换状态后抓取，迁移也由同一入口触发
  `relocate_storage`；截图完成后临时入口已整段删除，源码零残留。`rfd` 原生目录选择器未纳入截图。
- **迁移实测**：在真实窗口执行迁移到 `target/gui-012/migrated`，`migrated/` 得到 `data.db` 与
  `attachments/7/*.png`；旧位置只剩 `app.lock` 与 `location`；重启后状态栏显示新路径，待办、
  备注与图片缩略图均来自新位置。

### 门禁（T019）

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --all-targets --all-features` | PASS（lib 62 + 集成 86） |
| `cargo machete` | PASS，无未使用依赖 |
| `cargo audit` | PASS（5 条 allowed warnings，与 `main` 一致） |
| `git grep 'Color32::' -- src` | 仅 `src/app/theme.rs`（009 令牌文件） |
| `git grep '#\[(allow\|expect)(' -- src` | 无命中，无 lint 抑制 |
| 裸 `TODO` / `FIXME` | 源码无命中 |
| 文件规模（宪法 I） | 最大 `src/app.rs` 617 行，其余全部 < 600 行 |

### 宪法复核

| 原则 | 结论 |
| --- | --- |
| I 文件规模 | PASS（均 < 700）。`src/app.rs` 617 行已过 600 预警线，下次往该文件新增代码前必须先拆分 |
| II 质量 | PASS：无死代码、无注释代码、无裸 TODO、无 lint 抑制 |
| III 测试先行 | PASS，见「测试先行」；缺陷修复同样先红后绿 |
| IV 接触面 | PASS：迁移与配置错误文案统一说明「哪一项不合法、影响范围、如何修正」 |
| V 性能 | 未改动调度热路径；`benches/plan_day.rs` 仅补 `note` 字段，基准输入与逻辑不变，按无差异处理 |
| VI 简单性 | PASS：无新增 trait／开关／配置；临时预览入口与探针脚本未入库 |
| VII 可观测性 | PASS：迁移、附件、备注入口均带 `op` / `result` / `elapsed_ms` 字段；错误只记一次 |
| VIII 安全 | PASS：图片在边界校验格式与大小，路径拼装受附件根约束，SQL 全部参数绑定 |

### 待跟进

- `src/app.rs` 拆分（617 行，过预警线）：建议把 `startup`、`reload`、`sync_popup` 与事件循环
  拆成子模块，作为独立提交，不夹带功能改动。
- 交互响应与后台驻留内存仍无测量环境（宪法 V 已登记），补齐前不作门禁。
