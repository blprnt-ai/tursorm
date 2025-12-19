#![deny(warnings)]

pub(crate) mod connection;
pub(crate) mod error;
pub(crate) mod query;
pub(crate) mod traits;
pub(crate) mod value;

pub mod migration;

pub mod prelude;
pub use prelude::*;
pub use traits::entity::EntityDeleteExt;
pub use traits::entity::EntitySelectExt;
pub use traits::model::ModelDeleteExt;
