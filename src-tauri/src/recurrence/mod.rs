//! Recurrence expansion and edit scopes. See docs/03-modelo-de-datos.md sections 3 and 4.

pub mod edit_scope;
pub mod expand;

pub use edit_scope::EditScope;
pub use expand::{expand_master, materialize_simple, Window};
