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
- [ ] T009 今日页：最小行动列表、完成/跳过/撤销、备注入口（对应 012/013）
- [ ] T010 历史页：历史行动分页加载与结算（对应 012）

## Phase 3：任务内容与备注两栏（TDD）

- [ ] T011 任务内容对话框：多行文本输入 + 媒体（图片/音频/视频）增删（对应 013）
- [ ] T012 备注两栏：来源任务（标题+富内容）/ 当前备注（对应 013）
- [ ] T013 富内容渲染：链接可点击、图片内联、音视频「打开」（对应 013）

## Phase 4：其余对话框与栏（TDD）

- [ ] T014 重命名/删除/频率/耗时/存储/统计 对话框迁移（逐个 TDD）
- [ ] T015 工具栏（每日上限/主题/统计入口）、菜单栏、状态栏迁移（主题映射 kit API）
- [ ] T016 把现有 `egui::Context` headless 测试（`src/app/tests/*`）改写为 gpui-kit 测试并转绿，
      删除全部 egui 专属测试代码

## Phase 5：收尾

- [ ] T017 门禁：`fmt --check` / `clippy -D warnings` / `test --all-targets --all-features` /
      `machete` / `audit`；复核宪法 I–VIII 与颜色字面量/`#[allow]`
- [ ] T018 隔离 `%LOCALAPPDATA%` 起 GUI，深浅截图验证外观与三页签/计数（标注本会话是否执行）
- [ ] T019 勾选本清单、填写验收记录，按逻辑拆分提交并开 PR（含宪法复核与单开发者仓库自我证明）
