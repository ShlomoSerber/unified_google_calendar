//! Synchronization engine. See docs/02-arquitectura.md section 2 (sync), section 3 and
//! docs/05-sincronizacion.md.

pub mod calendar_list;
pub mod ctx;
pub mod engine;
pub mod full;
pub mod holidays;
pub mod incremental;
pub mod map;

pub use ctx::SyncCtx;
pub use engine::{start, sync_all, Engine, SyncTick};
pub use full::full_sync_calendar;
pub use incremental::incremental_calendar;
