//! Reminder notifications. See docs/06-integracion-gnome.md section 1 and docs/03 section 8.

pub mod notify;
pub mod scheduler;

pub use scheduler::start;
