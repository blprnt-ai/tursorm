use super::active_model::ActiveModelTrait;
use super::column::ColumnTrait;
use super::from_row::FromRow;
use super::model::ModelTrait;

/// Trait for entity types that represent database tables
///
/// An entity defines the mapping between a Rust type and a database table.
/// It provides metadata about the table name, columns, and associated types.
///
/// This trait is typically implemented via the `#[derive(Entity)]` macro.
///
/// # Associated Types
///
/// - `Model` - The struct representing a row (used for SELECT results)
/// - `Column` - The enum representing table columns
/// - `ActiveModel` - The mutable struct for INSERT/UPDATE operations
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Debug, Entity)]
/// #[tursorm(table_name = "users")]
/// pub struct User {
///     #[tursorm(primary_key, auto_increment)]
///     pub id: i64,
///     pub name: String,
///     pub email: String,
/// }
///
/// // Query records
/// let users = User::find().all(&conn).await?;
///
/// // Insert a new record
/// let mut new_user = UserEntity::active_model();
/// new_user.name = set("Alice".to_string());
/// let user = new_user.insert(&conn).await?;
///
/// // Update a record
/// let mut active = user.into_active_model();
/// active.name = set("Alice Updated".to_string());
/// let user = active.update(&conn).await?;
///
/// // Delete a record
/// user.into_active_model().delete(&conn).await?;
/// ```
pub trait EntityTrait: Default + Send + Sync + 'static {
    /// The model type for this entity
    type Model: ModelTrait<Entity = Self> + FromRow + Send;

    /// The column enum type for this entity
    type Column: ColumnTrait;

    /// The active model type for this entity
    type ActiveModel: ActiveModelTrait<Entity = Self>;

    /// Get the table name
    fn table_name() -> &'static str;

    /// Get the primary key column
    fn primary_key() -> Self::Column;

    /// Check if primary key is auto-increment
    fn primary_key_auto_increment() -> bool;

    /// Get all columns as a comma-separated string
    fn all_columns() -> &'static str;

    /// Get the number of columns
    fn column_count() -> usize;
}
