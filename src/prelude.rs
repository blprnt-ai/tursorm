//! Prelude module for tursorm
//!
//! This module re-exports the most commonly used types and traits.
//!
//! ```ignore
//! use tursorm::prelude::*;
//! ```

// Re-export the derive macro
pub use turso::EncryptionOpts;
pub use tursorm_macros::Entity;

pub use crate::connection::Builder;
pub use crate::connection::Connection;
pub use crate::entity::ActiveModelTrait;
pub use crate::entity::ActiveValue;
pub use crate::entity::ColumnTrait;
pub use crate::entity::EntityTrait;
pub use crate::entity::FromRow;
pub use crate::entity::ModelTrait;
pub use crate::entity::not_set;
pub use crate::entity::set;
pub use crate::error::Error;
pub use crate::error::Result;
pub use crate::query::Condition;
pub use crate::query::Delete;
pub use crate::query::Insert;
pub use crate::query::InsertMany;
pub use crate::query::Order;
pub use crate::query::Select;
pub use crate::query::Update;
pub use crate::value::ColumnType;
pub use crate::value::FromValue;
pub use crate::value::IntoValue;
// Re-export optional types
#[cfg(feature = "with-json")]
pub use crate::value::Json;
pub use crate::value::Value;
