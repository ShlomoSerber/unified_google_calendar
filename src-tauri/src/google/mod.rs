//! Google Calendar API v3 client. See docs/02-arquitectura.md section 2 (google) and
//! docs/05-sincronizacion.md section 2. Only this module knows Google URLs, parameters and JSON.

pub mod body;
pub mod calendar_list;
pub mod channels;
pub mod client;
pub mod colors;
pub mod events;
pub mod types;

pub use client::Client;
