//! Push receiver. See docs/05-sincronizacion.md section 3.

pub mod server;
pub mod verify_file;

pub use server::{router, start, WebhookState};
