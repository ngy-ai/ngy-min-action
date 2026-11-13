//! 结构树对话框开启逻辑（重命名/删除/频率/统计/存储/备注/重置）。
//!
//! 这些 `MinActionApp` 方法把对应子对话框挂到根视图的 `dialog` 槽位；数据写回仍由子对话框自身完成。

use gpui_kit::*;
use tracing::error;

use super::note_dialog::NoteContext;
use super::task_editor::TaskEditor;
use super::{
    MinActionApp, StructDialog, frequency_dialog, note_dialog, reset_dialog, stats_dialog,
    storage_dialog,
};
use crate::model::{ActionId, Frequency, TaskId};

impl MinActionApp {
    /// 打开频率编辑对话框：依据当前频率预选种类，构造对话框实体并挂到根视图。
    pub(crate) fn open_frequency_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        editor: Entity<TaskEditor>,
        current: Frequency,
        window: &mut Window,
        cx: &mut App,
    ) {
        let dialog = cx.new(|c| {
            frequency_dialog::FrequencyDialog::new(editor, root.clone(), current, window, c)
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Frequency(dialog));
            cx.notify();
        });
    }

    /// 打开复习统计对话框：从数据层算出 `ReviewStats` 后构造对话框挂在根视图。
    pub(crate) fn open_stats(
        &mut self,
        root: Entity<MinActionApp>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let today = self.clock.today();
        let stats = match self
            .store
            .review_stats(today, stats_dialog::STATS_WINDOW_DAYS)
        {
            Ok(stats) => stats,
            Err(e) => {
                error!("读取复习统计失败：{e:#}");
                return;
            }
        };
        let dialog = cx.new(|_c| stats_dialog::StatsDialog::new(stats, root.clone()));
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Stats(dialog));
            cx.notify();
        });
    }

    /// 打开存储对话框：从数据目录递归算出占用后构造对话框挂在根视图。
    pub(crate) fn open_storage(
        &mut self,
        root: Entity<MinActionApp>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let info = storage_dialog::collect_storage(self.store.data_dir());
        let dialog = cx.new(|_c| storage_dialog::StorageDialog::new(info, root.clone()));
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Storage(dialog));
            cx.notify();
        });
    }

    /// 打开行动备注对话框：用来源任务上下文与既有备注构造两栏对话框挂在根视图。
    pub(crate) fn open_note_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        action_id: ActionId,
        context: NoteContext,
        window: &mut Window,
        cx: &mut App,
    ) {
        let dialog = cx.new(|c| {
            note_dialog::NoteDialog::new(
                context,
                action_id,
                self.store.clone(),
                root.clone(),
                window,
                c,
            )
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Note(dialog));
            cx.notify();
        });
    }

    /// 打开重置复习进度对话框：依据任务名构造确认面板挂在根视图。
    pub(crate) fn open_reset_dialog(
        &mut self,
        root: Entity<MinActionApp>,
        task_id: TaskId,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let task_name = self
            .store
            .task(task_id)
            .map(|task| task.name)
            .unwrap_or_default();
        let dialog = cx.new(|_c| {
            reset_dialog::ResetDialog::new(task_id, task_name, self.store.clone(), root.clone())
        });
        root.update(cx, |this, cx| {
            this.dialog = Some(StructDialog::Reset(dialog));
            cx.notify();
        });
    }
}
