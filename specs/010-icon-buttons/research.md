# Research: 用图标替换操作按钮

**Feature**: `010-icon-buttons` | **Date**: 2026-11-05

依据 `spec.md`、009 的视觉令牌与 egui 0.37 本地源码；不依赖网络资料。

## D1: 图标怎么呈现

**Decision**: 新增 `src/app/icons.rs`，用 `impl egui::Widget` 自定义控件，通过 `Painter::line_segment` / `circle_stroke` / `rect_stroke` 画简单线条形状；`IconButton` 持有图标种类、悬停文案与语义级别（普通／主要／危险）。

**Rationale**: 完全不依赖字体是否包含某个符号，杜绝「豆腐块」（FR-009）；不引入图标库、字体、图片或网络资源（FR-003、FR-011、009 的 FR-014 继续有效）；形状用当前 `ui.visuals()` 的文本色或语义色绘制，两套主题自动跟随（FR-004）。

**Alternatives considered**:
- Unicode 文本符号（✎ / ⏸ / ⟳ / ⚙）：实现最省事，但内置字体是否覆盖这些码位不可控，缺字会直接破坏界面；若逐个截图验证再回退，成本与矢量绘制相当
- 引入 `egui-phosphor` / `lucide` 等图标库：违反「完全离线、不新增依赖」的既有约束，且与 009 建立的语义令牌体系并列成第二套颜色来源
- 内嵌 PNG/SVG 资源：需要图片加载与入库资源，同样违反离线约束

## D2: 图标集与按钮映射

**Decision**: 13 种形状覆盖全部范围内的按钮，形状与文案的对应关系集中在 `contracts/icon-buttons.md`；同一形状可用于不同按钮，靠悬停文案区分语义。

**Rationale**: 形状少、易画、易辨认；集中映射满足 FR-012，避免各视图散落重复的字面量。

**Alternatives considered**: 每个按钮一种独特形状：形状过多、线条相近反而难辨认，也放大绘制代码。

## D3: 可发现性

**Decision**: 每个 `IconButton` 都调用 `Response::on_hover_text(中文原文)`；危险操作额外保留 009 的二次确认流程；主题三态下拉、表单标签、频率/耗时选项、统计数值、空状态与错误提示保留文字。

**Rationale**: 满足 FR-002 与 FR-008；这些控件承载文字信息而非单一动作，图标化会丢失信息。

## D4: 命中区域与布局

**Decision**: `IconButton` 统一 `allocate_exact_size(egui::vec2(34.0, 26.0), Sense::click())`，绘制区域留出内边距；悬停时画一个圆角底色，给出按钮反馈。

**Rationale**: 34×26 的命中面积不小于原 `small_button`（约 20×18）与多数两字文字按钮，满足 FR-007；固定尺寸让任务行列宽可预期，窄窗口下不会挤压（FR-009）。

**Alternatives considered**: 沿用文字按钮自适应宽度：各按钮宽度不一致，任务行反而更宽，失去图标化带来的紧凑性。

## D5: 与 009 令牌的关系

**Decision**: 图标颜色只取 `ui.visuals().text_color()` 与 `error_fg_color`；线宽用 1.6（普通）/ 2.2（主要、危险）区分；不新增 `Color32` 字面量。009 的 `theme::danger` 与 `theme::primary_button` 在替换完成后若无消费者则删除，避免死代码。

**Rationale**: 宪法 II 禁止死代码；FR-004 要求单一颜色来源。

## D6: 是否需要新增依赖或数据

**Decision**: 不需要。`Cargo.toml` 不变，不改任何业务表、设置键或调度逻辑。

**Rationale**: FR-010、FR-011、宪法 VI。

## D7: 文件规模

**Decision**: 新文件 `src/app/icons.rs` 控制在 250 行内；各视图只把一行按钮调用换成一行 `ui.add(IconButton::new(...))`，`src/app/main_view.rs` 等不会越过 600 行预警线。

**Rationale**: 宪法 I；替换不引入新段落，行数基本持平。
