//! Edit scopes for recurring events. Implemented in F1-T3. See docs/03 section 4.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditScope {
    This,
    Following,
    All,
}
