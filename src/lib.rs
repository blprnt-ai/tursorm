#![deny(warnings)]

pub(crate) mod connection;
pub(crate) mod error;
pub(crate) mod query;
pub(crate) mod traits;
pub(crate) mod value;

pub mod migration;

pub mod prelude;
pub use prelude::*;
pub use traits::record::RecordDeleteExt;
pub use traits::table::TableDeleteExt;
pub use traits::table::TableSelectExt;
