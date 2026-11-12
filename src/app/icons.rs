//! 图标按钮：用线条形状替换文字按钮，形状与提示文案集中在此处。
//!
//! 不依赖字体是否包含某个符号，也不引入图标库、字体、图片或网络资源；
//! 颜色只取当前主题的 `text_color()` 与 `error_fg_color()`。

use eframe::egui;

/// 图标形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Plus,
    Minus,
    ChevronRight,
    ChevronDown,
    Pencil,
    Cross,
    Check,
    Skip,
    CircularArrow,
    Pause,
    Play,
    Chart,
    List,
    Note,
}

/// 按钮语义：决定线宽与颜色来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Semantic {
    Normal,
    Primary,
    Danger,
}

/// 每个操作按钮的唯一标识；调用点只用这个枚举，不散落形状与文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonId {
    AddGoal,
    AddCategory,
    AddTask,
    Create,
    Save,
    Confirm,
    OpenStats,
    ViewToday,
    ExpandAll,
    CollapseAll,
    ChevronRight,
    ChevronDown,
    Rename,
    DeleteTask,
    TaskDetail,
    DeleteConfirm,
    Pause,
    Resume,
    ResetProgress,
    ResetConfirm,
    Cancel,
    Close,
    Done,
    Skip,
    Undo,
    Note,
    RemoveImage,
}

impl ButtonId {
    /// 全部按钮，供映射表校验与双主题渲染用例遍历。
    pub(crate) const ALL: &[Self] = &[
        Self::AddGoal,
        Self::AddCategory,
        Self::AddTask,
        Self::Create,
        Self::Save,
        Self::Confirm,
        Self::OpenStats,
        Self::ViewToday,
        Self::ExpandAll,
        Self::CollapseAll,
        Self::ChevronRight,
        Self::ChevronDown,
        Self::Rename,
        Self::DeleteTask,
        Self::TaskDetail,
        Self::DeleteConfirm,
        Self::Pause,
        Self::Resume,
        Self::ResetProgress,
        Self::ResetConfirm,
        Self::Cancel,
        Self::Close,
        Self::Done,
        Self::Skip,
        Self::Undo,
        Self::Note,
        Self::RemoveImage,
    ];
}

/// 按钮的形状、悬停文案与语义；与 `contracts/icon-buttons.md` 一一对应。
pub(crate) struct ButtonSpec {
    pub(crate) icon: Icon,
    pub(crate) tip: &'static str,
    pub(crate) semantic: Semantic,
}

/// 查询按钮映射；这是形状与文案的唯一数据源。
pub(crate) const fn spec(id: ButtonId) -> ButtonSpec {
    use ButtonId as Id;
    use Semantic as S;
    match id {
        Id::AddGoal => ButtonSpec {
            icon: Icon::Plus,
            tip: "添加目标",
            semantic: S::Normal,
        },
        Id::AddCategory => ButtonSpec {
            icon: Icon::Plus,
            tip: "添加分类",
            semantic: S::Normal,
        },
        Id::AddTask => ButtonSpec {
            icon: Icon::Plus,
            tip: "添加任务",
            semantic: S::Primary,
        },
        Id::Create => ButtonSpec {
            icon: Icon::Check,
            tip: "创建",
            semantic: S::Primary,
        },
        Id::Save => ButtonSpec {
            icon: Icon::Check,
            tip: "保存",
            semantic: S::Primary,
        },
        Id::Confirm => ButtonSpec {
            icon: Icon::Check,
            tip: "知道了",
            semantic: S::Normal,
        },
        Id::OpenStats => ButtonSpec {
            icon: Icon::Chart,
            tip: "统计",
            semantic: S::Normal,
        },
        Id::ViewToday => ButtonSpec {
            icon: Icon::List,
            tip: "查看今日清单",
            semantic: S::Primary,
        },
        Id::ExpandAll => ButtonSpec {
            icon: Icon::Plus,
            tip: "全部展开",
            semantic: S::Normal,
        },
        Id::CollapseAll => ButtonSpec {
            icon: Icon::Minus,
            tip: "全部折叠",
            semantic: S::Normal,
        },
        Id::ChevronRight => ButtonSpec {
            icon: Icon::ChevronRight,
            tip: "展开",
            semantic: S::Normal,
        },
        Id::ChevronDown => ButtonSpec {
            icon: Icon::ChevronDown,
            tip: "折叠",
            semantic: S::Normal,
        },
        Id::Rename => ButtonSpec {
            icon: Icon::Pencil,
            tip: "改名",
            semantic: S::Normal,
        },
        Id::DeleteTask => ButtonSpec {
            icon: Icon::Cross,
            tip: "删除任务",
            semantic: S::Danger,
        },
        Id::TaskDetail => ButtonSpec {
            icon: Icon::Note,
            tip: "任务内容",
            semantic: S::Normal,
        },
        Id::DeleteConfirm => ButtonSpec {
            icon: Icon::Cross,
            tip: "删除",
            semantic: S::Danger,
        },
        Id::Pause => ButtonSpec {
            icon: Icon::Pause,
            tip: "暂停",
            semantic: S::Normal,
        },
        Id::Resume => ButtonSpec {
            icon: Icon::Play,
            tip: "恢复",
            semantic: S::Normal,
        },
        Id::ResetProgress => ButtonSpec {
            icon: Icon::CircularArrow,
            tip: "重置复习",
            semantic: S::Normal,
        },
        Id::ResetConfirm => ButtonSpec {
            icon: Icon::CircularArrow,
            tip: "重置复习",
            semantic: S::Danger,
        },
        Id::Cancel => ButtonSpec {
            icon: Icon::Cross,
            tip: "取消",
            semantic: S::Normal,
        },
        Id::Close => ButtonSpec {
            icon: Icon::Cross,
            tip: "关闭",
            semantic: S::Normal,
        },
        Id::Done => ButtonSpec {
            icon: Icon::Check,
            tip: "完成",
            semantic: S::Normal,
        },
        Id::Skip => ButtonSpec {
            icon: Icon::Skip,
            tip: "跳过",
            semantic: S::Normal,
        },
        Id::Undo => ButtonSpec {
            icon: Icon::CircularArrow,
            tip: "撤销",
            semantic: S::Normal,
        },
        Id::Note => ButtonSpec {
            icon: Icon::Note,
            tip: "备注",
            semantic: S::Normal,
        },
        Id::RemoveImage => ButtonSpec {
            icon: Icon::Cross,
            tip: "移除图片",
            semantic: S::Normal,
        },
    }
}

/// 把一个图标按钮加入当前界面，返回可继续调用 `.clicked()` 的响应。
pub(crate) fn add(ui: &mut egui::Ui, id: ButtonId) -> egui::Response {
    debug_assert!(ButtonId::ALL.contains(&id), "按钮映射表缺少 {id:?}");
    ui.add(IconButton::new(id))
}

/// 自绘图标按钮：固定命中区域、悬停提示、按语义取色。
pub(crate) struct IconButton {
    id: ButtonId,
}

impl IconButton {
    pub(crate) const fn new(id: ButtonId) -> Self {
        Self { id }
    }
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let button = spec(self.id);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 26.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let visuals = ui.visuals();
            let color = if button.semantic == Semantic::Danger {
                visuals.error_fg_color
            } else {
                visuals.text_color()
            };
            let hovered = response.hovered() || response.has_focus();
            let width = if button.semantic == Semantic::Normal {
                1.6
            } else {
                2.2
            };
            let stroke = egui::Stroke::new(width + if hovered { 0.6 } else { 0.0 }, color);
            if hovered {
                ui.painter()
                    .rect_filled(rect, 4.0, visuals.widgets.hovered.bg_fill);
            }
            draw_icon(ui.painter(), button.icon, rect, stroke);
        }
        response.on_hover_text(button.tip)
    }
}

fn draw_icon(painter: &egui::Painter, icon: Icon, rect: egui::Rect, stroke: egui::Stroke) {
    let center = rect.center();
    let unit = rect.height() * 0.28;
    let point = |x: f32, y: f32| egui::pos2(center.x + x * unit, center.y + y * unit);
    let line = |from: (f32, f32), to: (f32, f32)| {
        painter.line_segment([point(from.0, from.1), point(to.0, to.1)], stroke);
    };

    match icon {
        Icon::Plus => {
            line((-1.0, 0.0), (1.0, 0.0));
            line((0.0, -1.0), (0.0, 1.0));
        }
        Icon::Minus => line((-1.0, 0.0), (1.0, 0.0)),
        Icon::ChevronRight => {
            line((-0.5, -1.0), (0.5, 0.0));
            line((0.5, 0.0), (-0.5, 1.0));
        }
        Icon::ChevronDown => {
            line((-1.0, -0.5), (0.0, 0.5));
            line((0.0, 0.5), (1.0, -0.5));
        }
        Icon::Pencil => {
            line((-1.0, 1.0), (0.6, -0.9));
            line((0.6, -0.9), (1.0, -0.5));
            line((-1.0, 1.0), (-0.6, 1.0));
        }
        Icon::Cross => {
            line((-0.8, -0.8), (0.8, 0.8));
            line((-0.8, 0.8), (0.8, -0.8));
        }
        Icon::Check => {
            line((-1.0, 0.0), (-0.2, 0.8));
            line((-0.2, 0.8), (1.0, -0.8));
        }
        Icon::Skip => {
            line((-0.9, -0.8), (0.3, 0.0));
            line((0.3, 0.0), (-0.9, 0.8));
            line((-0.9, -0.8), (-0.9, 0.8));
            line((0.7, -0.8), (0.7, 0.8));
        }
        Icon::CircularArrow => {
            line((-0.8, 0.3), (-0.8, -0.6));
            line((-0.8, -0.6), (0.8, -0.6));
            line((0.8, -0.6), (0.8, 0.5));
            line((0.4, 0.1), (0.8, 0.5));
            line((1.0, 0.0), (0.8, 0.5));
        }
        Icon::Pause => {
            line((-0.5, -0.9), (-0.5, 0.9));
            line((0.5, -0.9), (0.5, 0.9));
        }
        Icon::Play => {
            line((-0.6, -0.9), (0.9, 0.0));
            line((0.9, 0.0), (-0.6, 0.9));
            line((-0.6, -0.9), (-0.6, 0.9));
        }
        Icon::Chart => {
            line((-1.0, 1.0), (1.0, 1.0));
            line((-0.6, 1.0), (-0.6, 0.1));
            line((0.0, 1.0), (0.0, -0.6));
            line((0.6, 1.0), (0.6, -0.2));
        }
        Icon::List => {
            line((-0.9, -0.6), (0.9, -0.6));
            line((-0.9, 0.0), (0.9, 0.0));
            line((-0.9, 0.6), (0.9, 0.6));
        }
        Icon::Note => {
            line((-0.7, -0.9), (0.7, -0.9));
            line((0.7, -0.9), (0.7, 0.9));
            line((0.7, 0.9), (-0.7, 0.9));
            line((-0.7, 0.9), (-0.7, -0.9));
            line((-0.4, -0.2), (0.4, -0.2));
            line((-0.4, 0.3), (0.4, 0.3));
        }
    }
}
