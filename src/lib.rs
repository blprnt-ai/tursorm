#![deny(warnings)]

//! # tursorm
//!
//! A simple ORM for [Turso](https://turso.tech) inspired by SeaORM.
//!
//! ## Features
//!
//! - Derive macro for defining entities
//! - Type-safe query builders (Select, Insert, Update, Delete)
//! - Fluent API with method chaining
//! - Support for common SQL operations (filtering, ordering, pagination)
//! - Optional support for chrono, uuid, and JSON types
//!
//! ## Quick Start
//!
//! ```ignore
//! use tursorm::prelude::*;
//!
//! // Define an entity
//! #[derive(Clone, Debug, Entity)]
//! #[tursorm(table_name = "users")]
//! pub struct User {
//!     #[tursorm(primary_key, auto_increment)]
//!     pub id: i64,
//!     pub name: String,
//!     pub email: String,
//!     pub created_at: Option<String>,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Connect to database
//!     let db = Builder::new_local(":memory:").build().await?;
//!     let conn = db.connect()?;
//!
//!     // Create table
//!     conn.execute(
//!         "CREATE TABLE users (
//!             id INTEGER PRIMARY KEY AUTOINCREMENT,
//!             name TEXT NOT NULL,
//!             email TEXT NOT NULL,
//!             created_at TEXT
//!         )",
//!         ()
//!     ).await?;
//!
//!     // Insert a new user
//!     let mut new_user = UserEntity::active_model();
//!     new_user.name = set("Alice".to_string());
//!     new_user.email = set("alice@example.com".to_string());
//!     let user = new_user.insert(&conn).await?;
//!
//!     // Find users
//!     let users = User::find()
//!         .filter(Condition::eq(UserColumn::Name, "Alice"))
//!         .all(&conn)
//!         .await?;
//!
//!     // Update a user
//!     let mut active = user.clone().into_active_model();
//!     active.name = set("Alice Smith".to_string());
//!     let user = active.update(&conn).await?;
//!
//!     // Delete a user
//!     user.into_active_model().delete(&conn).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Entity Attributes
//!
//! The `#[tursorm(...)]` attribute supports:
//!
//! - `table_name = "..."` - Set the table name (default: struct name in snake_case + 's')
//! - `primary_key` - Mark a field as the primary key
//! - `auto_increment` - Mark a primary key as auto-incrementing
//! - `column_name = "..."` - Set a custom column name
//!
//! ## Query Examples
//!
//! ### Select
//!
//! ```ignore
//! // Find all
//! let users = User::find().all(&conn).await?;
//!
//! // Find by ID
//! let user = User::find_by_id(1).one(&conn).await?;
//!
//! // With conditions
//! let users = User::find()
//!     .filter(Condition::eq(UserColumn::Name, "Alice"))
//!     .filter(Condition::is_not_null(UserColumn::Email))
//!     .order_by_desc(UserColumn::CreatedAt)
//!     .limit(10)
//!     .all(&conn)
//!     .await?;
//!
//! // Count
//! let count = User::find()
//!     .filter(Condition::contains(UserColumn::Email, "@example.com"))
//!     .count(&conn)
//!     .await?;
//! ```
//!
//! ### Insert
//!
//! ```ignore
//! let mut new_user = UserEntity::active_model();
//! new_user.name = set("Bob".to_string());
//! new_user.email = set("bob@example.com".to_string());
//!
//! // Insert and get the inserted row (recommended)
//! let user = new_user.insert(&conn).await?;
//!
//! // Insert and get row count only
//! new_user.insert_exec(&conn).await?;
//! ```
//!
//! ### Update
//!
//! ```ignore
//! let mut active = user.into_active_model();
//! active.name = set("Updated Name".to_string());
//!
//! // Update and get the updated row (recommended)
//! let user = active.update(&conn).await?;
//!
//! // Update and get row count only
//! active.update_exec(&conn).await?;
//!
//! // Bulk update (using query builder directly)
//! Update::<UserEntity>::many()
//!     .set(UserColumn::Name, "Anonymous")
//!     .filter(Condition::is_null(UserColumn::Email))
//!     .exec(&conn)
//!     .await?;
//! ```
//!
//! ### Delete
//!
//! ```ignore
//! // Delete a model
//! user.into_active_model().delete(&conn).await?;
//!
//! // Delete with condition (using query builder directly)
//! Delete::<UserEntity>::new()
//!     .filter(Condition::lt(UserColumn::CreatedAt, "2020-01-01"))
//!     .exec(&conn)
//!     .await?;
//! ```

pub(crate) mod connection;
pub(crate) mod error;
pub(crate) mod query;
pub(crate) mod schema;
pub(crate) mod traits;
pub(crate) mod value;

pub mod migration;

pub mod prelude;
pub use prelude::*;
pub use traits::entity::EntityDeleteExt;
pub use traits::entity::EntitySelectExt;
pub use traits::model::ModelDeleteExt;
