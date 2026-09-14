//! Recurrence expansion, edit scopes and text. See docs/03-modelo-de-datos.md sections 3, 4 and 9.

pub mod describe;
pub mod edit_scope;
pub mod expand;

pub use edit_scope::EditScope;
pub use expand::{expand_master, materialize_simple, Window};
