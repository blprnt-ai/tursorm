//! Entity and model traits for tursorm
//!
//! This module defines the core traits that enable the ORM functionality:
//!
//! - [`EntityTrait`] - Defines table metadata (name, columns, primary key)
//! - [`ModelTrait`] - Represents a database row as a Rust struct
//! - [`ActiveModelTrait`] - Mutable model for insert/update operations
//! - [`ColumnTrait`] - Column metadata (name, type, constraints)
//! - [`FromRow`] - Converts database rows to model instances
//!
//! These traits are typically implemented by the `#[derive(Entity)]` macro.

use crate::error::Result;
use crate::value::ColumnType;
use crate::value::Value;

/// Trait for column enum types that describe table columns
///
/// Each entity has an associated column enum that implements this trait.
/// The enum variants represent individual columns and provide metadata
/// like column name, type, and constraints.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Copy, Debug)]
/// pub enum UserColumn {
///     Id,
///     Name,
///     Email,
/// }
///
/// impl ColumnTrait for UserColumn {
///     fn name(&self) -> &'static str {
///         match self {
///             UserColumn::Id => "id",
///             UserColumn::Name => "name",
///             UserColumn::Email => "email",
///         }
///     }
///     // ... other methods
/// }
/// ```
pub trait ColumnTrait: Copy + Clone + std::fmt::Debug + std::fmt::Display + 'static {
    /// Get the column name
    fn name(&self) -> &'static str;

    /// Get the column type
    fn column_type(&self) -> ColumnType;

    /// Check if this column is nullable
    fn is_nullable(&self) -> bool {
        false
    }

    /// Check if this column is a primary key
    fn is_primary_key(&self) -> bool {
        false
    }

    /// Check if this column is auto-increment
    fn is_auto_increment(&self) -> bool {
        false
    }

    /// Get the default value SQL expression (if any)
    fn default_value(&self) -> Option<&'static str> {
        None
    }

    /// Check if this column is unique
    fn is_unique(&self) -> bool {
        false
    }

    /// Get all columns as a static slice
    fn all() -> &'static [Self];
}

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
/// ```
pub trait EntityTrait: Default + Send + Sync {
    /// The model type for this entity
    type Model: ModelTrait<Entity = Self> + FromRow;

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

/// Trait for model types that represent database rows
///
/// A model is an immutable struct that holds the data from a database row.
/// It's the result type when querying with SELECT operations.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Debug)]
/// pub struct User {
///     pub id: i64,
///     pub name: String,
///     pub email: String,
/// }
///
/// impl ModelTrait for User {
///     type Entity = UserEntity;
///     
///     fn get_primary_key_value(&self) -> Value {
///         Value::Integer(self.id)
///     }
/// }
/// ```
pub trait ModelTrait: Clone {
    /// The entity type for this model
    type Entity: EntityTrait;

    /// Get the primary key value
    fn get_primary_key_value(&self) -> Value;
}

/// Trait for converting from a database row to a model
///
/// This trait enables automatic deserialization of query results into
/// Rust structs. It's implemented for model types to extract column
/// values from a database row.
///
/// # Errors
///
/// Implementations should return errors for:
/// - Missing required columns
/// - Type conversion failures
/// - Unexpected null values in non-nullable fields
pub trait FromRow: Sized {
    /// Convert from a database row
    ///
    /// # Errors
    ///
    /// Returns an error if the row cannot be converted to this type.
    fn from_row(row: &turso::Row) -> Result<Self>;
}

/// Trait for active model types used in INSERT and UPDATE operations
///
/// An active model is a mutable version of a model where each field is
/// wrapped in [`ActiveValue`]. This allows tracking which fields have been
/// explicitly set, so only those fields are included in queries.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Debug, Default)]
/// pub struct UserActiveModel {
///     pub id: ActiveValue<i64>,
///     pub name: ActiveValue<String>,
///     pub email: ActiveValue<String>,
/// }
///
/// // Only set fields are included in the INSERT
/// let new_user = UserActiveModel {
///     name: set("Alice".to_string()),
///     email: set("alice@example.com".to_string()),
///     ..Default::default()  // id is NotSet, will use auto-increment
/// };
/// ```
pub trait ActiveModelTrait: Default + Clone + Send + Sync {
    /// The entity type for this active model
    type Entity: EntityTrait;

    /// Get columns and values for insert
    fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>);

    /// Get column-value pairs for update (excluding primary key)
    fn get_update_sets(&self) -> Vec<(&'static str, Value)>;

    /// Get the primary key value if set
    fn get_primary_key_value(&self) -> Option<Value>;

    /// Get the primary key column name
    fn primary_key_column() -> &'static str;
}

/// Active value wrapper for tracking field state in active models
///
/// `ActiveValue` wraps field values in active models to track whether
/// a field has been explicitly set. Only `Set` values are included in
/// INSERT and UPDATE queries.
///
/// # Variants
///
/// - `Set(T)` - The field has a value and will be included in queries
/// - `NotSet` - The field has no value and will be excluded from queries
///
/// # Example
///
/// ```ignore
/// use tursorm::{ActiveValue, set, not_set};
///
/// // Using the set() helper
/// let name: ActiveValue<String> = set("Alice".to_string());
///
/// // Using the not_set() helper
/// let id: ActiveValue<i64> = not_set();
///
/// // Using From trait
/// let age: ActiveValue<i32> = 25.into();
///
/// // Check state
/// assert!(name.is_set());
/// assert!(id.is_not_set());
/// ```
#[derive(Clone, Debug)]
pub enum ActiveValue<T> {
    /// Value is set and will be included in queries
    Set(T),
    /// Value is not set and will be excluded from queries
    NotSet,
}

impl<T> Default for ActiveValue<T> {
    fn default() -> Self {
        ActiveValue::NotSet
    }
}

impl<T> ActiveValue<T> {
    /// Create a new set value
    pub fn set(value: T) -> Self {
        ActiveValue::Set(value)
    }

    /// Check if value is set
    pub fn is_set(&self) -> bool {
        matches!(self, ActiveValue::Set(_))
    }

    /// Check if value is not set
    pub fn is_not_set(&self) -> bool {
        matches!(self, ActiveValue::NotSet)
    }

    /// Get a reference to the value if set
    ///
    /// Returns `None` if the value is `NotSet`.
    pub fn get(&self) -> Option<&T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    /// Take ownership of the value if set, consuming self
    ///
    /// Returns `None` if the value is `NotSet`.
    pub fn take(self) -> Option<T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    /// Unwrap the value, panicking if not set
    ///
    /// # Panics
    ///
    /// Panics with message "Called unwrap on NotSet ActiveValue" if
    /// the value is `NotSet`.
    pub fn unwrap(self) -> T {
        match self {
            ActiveValue::Set(v) => v,
            ActiveValue::NotSet => panic!("Called unwrap on NotSet ActiveValue"),
        }
    }
}

impl<T> From<T> for ActiveValue<T> {
    fn from(value: T) -> Self {
        ActiveValue::Set(value)
    }
}

/// Create a `Set` active value (shorthand for `ActiveValue::Set`)
///
/// This is the recommended way to set field values in active models.
///
/// # Example
///
/// ```ignore
/// let user = UserActiveModel {
///     name: set("Alice".to_string()),
///     email: set("alice@example.com".to_string()),
///     ..Default::default()
/// };
/// ```
pub fn set<T>(value: T) -> ActiveValue<T> {
    ActiveValue::Set(value)
}

/// Create a `NotSet` active value (shorthand for `ActiveValue::NotSet`)
///
/// Fields with `NotSet` values are excluded from INSERT and UPDATE queries.
///
/// # Example
///
/// ```ignore
/// let id: ActiveValue<i64> = not_set();
/// assert!(id.is_not_set());
/// ```
pub fn not_set<T>() -> ActiveValue<T> {
    ActiveValue::NotSet
}

#[cfg(test)]
mod tests {
    use super::*;

    // ActiveValue tests
    #[test]
    fn test_active_value_set() {
        let val = ActiveValue::Set(42);
        assert!(val.is_set());
        assert!(!val.is_not_set());
    }

    #[test]
    fn test_active_value_not_set() {
        let val: ActiveValue<i32> = ActiveValue::NotSet;
        assert!(!val.is_set());
        assert!(val.is_not_set());
    }

    #[test]
    fn test_active_value_default() {
        let val: ActiveValue<i32> = ActiveValue::default();
        assert!(val.is_not_set());
    }

    #[test]
    fn test_active_value_set_fn() {
        let val = ActiveValue::<i32>::set(42);
        assert!(val.is_set());
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_active_value_get_some() {
        let val = ActiveValue::Set(42);
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_active_value_get_none() {
        let val: ActiveValue<i32> = ActiveValue::NotSet;
        assert_eq!(val.get(), None);
    }

    #[test]
    fn test_active_value_take_some() {
        let val = ActiveValue::Set(42);
        assert_eq!(val.take(), Some(42));
    }

    #[test]
    fn test_active_value_take_none() {
        let val: ActiveValue<i32> = ActiveValue::NotSet;
        assert_eq!(val.take(), None);
    }

    #[test]
    fn test_active_value_unwrap_success() {
        let val = ActiveValue::Set(42);
        assert_eq!(val.unwrap(), 42);
    }

    #[test]
    #[should_panic(expected = "Called unwrap on NotSet ActiveValue")]
    fn test_active_value_unwrap_panic() {
        let val: ActiveValue<i32> = ActiveValue::NotSet;
        val.unwrap();
    }

    #[test]
    fn test_active_value_from() {
        let val: ActiveValue<i32> = 42.into();
        assert!(val.is_set());
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_set_helper() {
        let val = set(42);
        assert!(val.is_set());
        assert_eq!(val.unwrap(), 42);
    }

    #[test]
    fn test_not_set_helper() {
        let val: ActiveValue<i32> = not_set();
        assert!(val.is_not_set());
    }

    #[test]
    fn test_active_value_clone() {
        let val = ActiveValue::Set(42);
        let cloned = val.clone();
        assert!(cloned.is_set());
        assert_eq!(cloned.get(), Some(&42));
    }

    #[test]
    fn test_active_value_debug() {
        let set_val = ActiveValue::Set(42);
        let not_set_val: ActiveValue<i32> = ActiveValue::NotSet;

        assert!(format!("{:?}", set_val).contains("Set(42)"));
        assert!(format!("{:?}", not_set_val).contains("NotSet"));
    }

    // Test with different types
    #[test]
    fn test_active_value_with_string() {
        let val = set(String::from("hello"));
        assert!(val.is_set());
        assert_eq!(val.get(), Some(&String::from("hello")));
    }

    #[test]
    fn test_active_value_with_vec() {
        let val = set(vec![1, 2, 3]);
        assert!(val.is_set());
        assert_eq!(val.get(), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_active_value_with_option() {
        let val = set(Some(42));
        assert!(val.is_set());
        assert_eq!(val.get(), Some(&Some(42)));
    }

    // Test chained operations
    #[test]
    fn test_active_value_get_then_use() {
        let val = set(42);
        if let Some(v) = val.get() {
            assert_eq!(*v, 42);
        } else {
            panic!("Expected Some value");
        }
    }

    #[test]
    fn test_active_value_pattern_matching() {
        let val = set(42);
        match val {
            ActiveValue::Set(v) => assert_eq!(v, 42),
            ActiveValue::NotSet => panic!("Expected Set value"),
        }
    }

    #[test]
    fn test_not_set_pattern_matching() {
        let val: ActiveValue<i32> = not_set();
        match val {
            ActiveValue::Set(_) => panic!("Expected NotSet value"),
            ActiveValue::NotSet => {} // Success
        }
    }
}
