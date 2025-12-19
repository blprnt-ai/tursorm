use super::active_model::ActiveModelTrait;
use super::column::ColumnTrait;
use super::from_row::FromRow;
use super::model::ModelTrait;
use crate::Condition;
use crate::Delete;
use crate::IntoValue;
use crate::Select;

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

/// Extension trait for models to enable fluent querying
///
/// This trait is automatically implemented for all types that implement
/// [`ModelTrait`], providing convenient static methods for querying.
///
/// # Example
///
/// ```ignore
/// // Find all users
/// let users = User::find().all(&conn).await?;
///
/// // Find a user by ID
/// let user = User::find_by_id(1).one(&conn).await?;
/// ```
pub trait EntitySelectExt: EntityTrait {
    /// Create a SELECT query for all records
    fn find() -> Select<Self> {
        Select::new()
    }

    /// Create a SELECT query filtered by primary key
    fn find_by_id<V: crate::value::IntoValue>(id: V) -> Select<Self>
    where Self::Column: ColumnTrait {
        Select::new().filter(Condition::eq(<Self>::primary_key(), id))
    }
}

impl<E: EntityTrait> EntitySelectExt for E {}

pub trait EntityDeleteExt: EntityTrait {
    fn delete_many(models: Vec<Self::Model>) -> Delete<Self> {
        Delete::new()
            .filter(Condition::is_in(Self::primary_key(), models.iter().map(|m| m.get_primary_key_value()).collect()))
    }

    fn delete_many_by_ids<V: IntoValue>(ids: Vec<V>) -> Delete<Self> {
        Delete::new().filter(Condition::is_in(Self::primary_key(), ids))
    }

    fn truncate() -> Delete<Self> {
        Delete::new()
    }
}

impl<E: EntityTrait> EntityDeleteExt for E {}
