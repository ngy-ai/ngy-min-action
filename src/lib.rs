//! 最小行动管理系统的领域库。
//!
//! 本 crate 只放可测试的领域逻辑与存储；图形界面由 `ngy_min_action` 的 bin
//! target 通过 [`app`] 模块挂载。模块划分与职责见 `specs/001-min-action-core/plan.md`。

pub mod app;
pub mod attachment;
pub mod clock;
pub mod common;
pub mod config;
pub mod model;
pub mod notify;
pub mod palette;
pub mod review;
pub mod schedule;
pub mod storage;
pub mod store;
