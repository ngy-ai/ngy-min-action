# 015 任务清单

> 测试先行：每个任务先写 gpui-kit 失败测试，再写最少实现；转绿后再提交。
> 提交按逻辑拆分：规格/设计产物一个提交，每阶段实现一个提交。

## Phase 0：立项与调研

- [x] T001 写 015 `spec.md` / `plan.md` / `tasks.md`（本文件）
- [ ] T002 确认 gpui-kit 0.9.1 关键 API 实际签名：`application()` / `init` / `open_window`、
      `Render` trait、`component` 下的 `Button`/`TextInput`/`Tab`/`List`/`Img`/`Dialog`(Modal) 模块名、
      headless 测试宏；把结论回填 plan 的「组件映射表」
- [ ] T003 替换 `Cargo.toml` 依赖：移除 `eframe` / `egui_extras`，加入 `gpui-kit = "0.9"`；
      `cargo build` 触达新工具链（先红：删 egui 入口后无法编译，进入骨架重建）

## Phase 1：应用骨架（TDD）

- [ ] T004 入口改写：`main.rs` / `lib.rs` 用 `gpui_kit::application().run` + `init` + `open_window`，
      根视图 `MinActionApp`（gpui `View`）挂载；`Store` 以 `Arc<Store>` 注入
- [ ] T005 headless 启动测试：用 gpui-kit 测试宏构造 `AppContext`，创建根视图，断言窗口/根视图
      可渲染、无 panic
- [ ] T006 数据与状态桥接：根视图持有 `Arc<Store>` 与 UI 状态结构 + `reload()`（沿用 store 调用），
      从 egui 版 `MinActionApp` 平移逻辑，不改 `Store`；删除 egui 专属字段/方法

## Phase 2：结构树与三页签（TDD）

- [ ] T007 结构树（左栏）：目标/分类树 + 类别计数，选中回调刷新（对应 014 计数需求）
- [ ] T008 三页签容器：任务 / 今日待办 / 历史待办 切换（对应 014 三页签）
- [x] T009 今日页（TDD）：最小行动列表（scheduler `generate_day` 幂等生成，复用 `actions_on`/`daily_cap`）、
      完成/跳过/撤销（复用 `settle_action`/`undo_last_settlement`）；备注两栏已落地（T012）
- [x] T010 历史页（TDD）：分页加载历史行动（`actions_page` 首页 + 加载更多，复用 `HISTORY_PAGE_SIZE`），完成/跳过/撤销（复用 `settle_action`/`undo_last_settlement`，结算后就地更新内存保留分页）；按日期倒序展示任务名+日期；零数据层改动

## Phase 3：任务内容与备注两栏（TDD）

- [x] T011 任务内容对话框：点击任务打开，编辑名称 / 预计耗时(分钟) / 富文本内容，频率只读；媒体以 `Attachment` 卡片展示并支持增删（添加经 `rfd` 选文件拷入 `task_content/<task_id>/` 后写库，删除删库记录并清磁盘文件）；保存写回 Store 后 reload 并关闭
- [x] T012 备注两栏（TDD）：今日行动行「备注」按钮打开左右两栏对话框（左=来源任务标题+描述，右=当前备注文本域），
      保存经 `Store::set_action_note` 写库（空白规范为清除），非法内容经 `format_note_error` 给出可读错误且不关闭；零数据层改动
- [x] T013 富内容渲染：链接可点击（解析 markdown/bare 链接，点击 `cx.open_url` 打开系统浏览器）；已有媒体以 `Attachment` 卡片渲染——图片内联预览、音频/视频「打开」按钮（路径经 `Store::data_dir()/task_content/<task_id>/<file_name>` 构造 `file://` URI 打开）
      —— 媒体增删（T011b）已完成：增删核心逻辑抽成 `add_media_file`/`remove_media_file` 自由函数（写库 + 拷/删磁盘文件），未改动 `Store` 对外 API（符合数据层零改动）

## Phase 4：其余对话框与栏（TDD）

- [x] T014 重命名/删除对话框迁移（TDD）：结构树每行目标/分类旁加「重命名」「删除」按钮，
      重命名走 `rename_goal`/`rename_category`，删除走 `delete_goal`/`delete_category`（软删除，
      删除确认展示波及的分类/任务数，由根视图 `goal_deletion_impact`/`category_deletion_impact` 预算）
      —— 耗时在任务编辑器内已内联编辑故不再单独做弹窗；存储 对话框仍待做
- [x] T014 频率对话框迁移（TDD）：任务编辑器「频率」行加「编辑」按钮，打开居中对话框，
      可选 每天 / 每 N 天 / 每周 N 次 并填数值，确认写回 `TaskEditor.frequency`（经 `apply_edits`
      调 `update_task_frequency` 写库），非法数值给出可读错误且不关闭
- [x] T014 统计对话框迁移（TDD）：工具栏「统计」入口（`tab-bar` 右侧，顺带为 T015 铺底）打开只读面板，
      复用 `Store::review_stats` 展示窗口分项/完成率/连续天数/任务阶段分布，零数据层改动
- [x] T014 存储对话框迁移（TDD）：工具栏「存储」入口（tab-bar 右侧，顺带为 T015 铺底）打开只读面板，
      复用 `Store::data_dir` 并递归累加文件字节数展示数据目录与占用（人类可读），零数据层改动
- [x] T014 重置复习进度对话框（TDD）：补 plan 第 16 行遗留的 `reset_dialog.rs` 模块；任务编辑器「重置进度」
      按钮打开确认面板，确认经 `Store::reset_review_progress` 把遗忘曲线归零，任务不存在时保留对话框并提示「不存在」；零数据层改动
- [x] T015 工具栏/菜单栏/状态栏（TDD）：工具栏含每日上限步进（`set_daily_cap`，越界拒 `InvalidValue`）、主题切换（System/Light/Dark，持久化 `set_theme_choice` 并经 `Theme::change`/系统外观应用）、统计/存储入口；菜单栏（标题）、状态栏（上限/主题/今日进度/历史条数）；零数据层改动
- [x] T016 测试迁移收尾：原 `src/app/tests/*` 的 egui `Context` headless 测试目录在迁移早期随 egui 移除已删除；
      其行为由现 gpui-kit 测试接管——delete/frequency/rename/stats/storage/note/task_editor/rich_content 各 `tests.rs`
      与 `src/app/tests.rs`（reload/计数/今日/历史/主题/上限）。`reset.rs` 对应的 `reset_dialog` 此前漏实现，本任务补齐
      （含 `reset_dialog/tests.rs` 迁移 `resetting_review_progress_clears_level` 与 `resetting_missing_task_reports_not_found`）；
      `context_menu`/`icons` 属 egui 专属机制（改名/删除已由行内按钮覆盖、图标为视觉），其 egui 测试已随目录删除。全部 egui 测试代码已清除，套件转绿

## Phase 5：收尾

- [ ] T017 门禁：`fmt --check` / `clippy -D warnings` / `test --all-targets --all-features` /
      `machete` / `audit`；复核宪法 I–VIII 与颜色字面量/`#[allow]`
- [ ] T018 隔离 `%LOCALAPPDATA%` 起 GUI，深浅截图验证外观与三页签/计数（标注本会话是否执行）
- [ ] T019 勾选本清单、填写验收记录，按逻辑拆分提交并开 PR（含宪法复核与单开发者仓库自我证明）
