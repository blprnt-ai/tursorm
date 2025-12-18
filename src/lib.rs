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
//!     let new_user = UserActiveModel {
//!         name: set("Alice".to_string()),
//!         email: set("alice@example.com".to_string()),
//!         ..Default::default()
//!     };
//!     let user = User::insert(new_user).exec_with_returning(&conn).await?;
//!
//!     // Find users
//!     let users = User::find()
//!         .filter(Condition::eq(UserColumn::Name, "Alice"))
//!         .all(&conn)
//!         .await?;
//!
//!     // Update a user
//!     let mut update_model: UserActiveModel = user.clone().into();
//!     update_model.name = set("Alice Smith".to_string());
//!     User::update(update_model).exec(&conn).await?;
//!
//!     // Delete a user
//!     User::delete_by_id(user.id).exec(&conn).await?;
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
//! let new_user = UserActiveModel {
//!     name: set("Bob".to_string()),
//!     email: set("bob@example.com".to_string()),
//!     ..Default::default()
//! };
//!
//! // Insert and get row count
//! let affected = User::insert(new_user).exec(&conn).await?;
//!
//! // Insert and get the inserted row
//! let user = User::insert(new_user).exec_with_returning(&conn).await?;
//!
//! // Insert and get last insert ID
//! let id = User::insert(new_user).exec_with_last_insert_id(&conn).await?;
//! ```
//!
//! ### Update
//!
//! ```ignore
//! let mut update = UserActiveModel::from(user);
//! update.name = set("Updated Name".to_string());
//!
//! // Update by primary key
//! User::update(update).exec(&conn).await?;
//!
//! // Bulk update
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
//! // Delete by ID
//! User::delete_by_id(1).exec(&conn).await?;
//!
//! // Delete with condition
//! Delete::<UserEntity>::new()
//!     .filter(Condition::lt(UserColumn::CreatedAt, "2020-01-01"))
//!     .exec(&conn)
//!     .await?;
//! ```

pub mod connection;
pub mod entity;
pub mod error;
pub mod migration;
pub mod prelude;
pub mod query;
pub mod value;

// Re-export main types at crate root
pub use connection::Builder;
pub use connection::Connection;
pub use connection::ConnectionExt;
pub use connection::Database;
pub use connection::Schema;
pub use entity::ActiveModelTrait;
pub use entity::ActiveValue;
pub use entity::ColumnTrait;
pub use entity::EntityTrait;
pub use entity::FromRow;
pub use entity::ModelTrait;
pub use entity::not_set;
pub use entity::set;
pub use error::Error;
pub use error::Result;
pub use query::Condition;
pub use query::Delete;
pub use query::Insert;
pub use query::InsertMany;
pub use query::Order;
pub use query::OrderBy;
pub use query::Select;
pub use query::Update;
// Re-export turso types for convenience
pub use turso::Row;
// Re-export the derive macro
pub use tursorm_macros::Entity;
pub use value::ColumnType;
pub use value::FromValue;
pub use value::IntoValue;
// Re-export optional types
#[cfg(feature = "with-json")]
pub use value::Json;
pub use value::Value;
