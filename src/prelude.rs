//! Prelude module for tursorm
//!
//! This module re-exports the most commonly used types and traits.
//!
//! ```ignore
//! use tursorm::prelude::*;
//! ```

pub use turso::Row;
pub use turso::Rows;
pub use tursorm_macros::Entity;

pub use crate::connection::prelude::*;
pub use crate::error::Error;
pub use crate::error::Result;
pub use crate::query::prelude::*;
pub use crate::schema::Schema;
pub use crate::traits::prelude::*;
pub use crate::value::ColumnType;
pub use crate::value::FromValue;
pub use crate::value::IntoValue;
// Re-export optional types
#[cfg(feature = "with-json")]
pub use crate::value::Json;
pub use crate::value::Value;
