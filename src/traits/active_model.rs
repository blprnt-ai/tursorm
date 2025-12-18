use super::entity::EntityTrait;
use crate::error::Result;
use crate::value::Value;

/// Trait for active model types used in INSERT and UPDATE operations
///
/// An active model is a mutable version of a model where each field is
/// wrapped in [`ActiveValue`]. This allows tracking which fields have been
/// explicitly set, so only those fields are included in queries.
///
/// # Example
///
/// ```ignore
/// use tursorm::prelude::*;
///
/// // Create a new active model and insert it
/// let mut new_user = UserEntity::active_model();
/// new_user.name = set("Alice".to_string());
/// new_user.email = set("alice@example.com".to_string());
///
/// // Insert and get the inserted row back
/// let user = new_user.insert(&conn).await?;
///
/// // Update an existing record
/// let mut active = user.into_active_model();
/// active.name = set("Alice Updated".to_string());
/// let updated_user = active.update(&conn).await?;
///
/// // Delete a record
/// user.into_active_model().delete(&conn).await?;
/// ```
#[async_trait::async_trait]
pub trait ActiveModelTrait: Default + Clone + Send + Sync + Sized + 'static {
    /// The entity type for this active model
    type Entity: EntityTrait<ActiveModel = Self>;

    /// Get columns and values for insert
    fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>);

    /// Get column-value pairs for update (excluding primary key)
    fn get_update_sets(&self) -> Vec<(&'static str, Value)>;

    /// Get the primary key value if set
    fn get_primary_key_value(&self) -> Option<Value>;

    /// Get the primary key column name
    fn primary_key_column() -> &'static str;

    /// Insert this active model into the database and return the inserted row
    ///
    /// This is the primary way to insert new records. The returned model
    /// includes any auto-generated values (like auto-increment IDs).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut new_user = UserEntity::active_model();
    /// new_user.name = set("Alice".to_string());
    /// new_user.email = set("alice@example.com".to_string());
    ///
    /// let user = new_user.insert(&conn).await?;
    /// println!("Created user with ID: {}", user.id);
    /// ```
    async fn insert(self, conn: &crate::Connection) -> Result<<Self::Entity as EntityTrait>::Model>
    where <Self::Entity as EntityTrait>::Model: Send {
        let row_id = crate::query::Insert::<Self::Entity>::new(self).exec_with_last_insert_id(conn).await?;
        let row = crate::query::Select::<Self::Entity>::new()
            .filter(crate::query::Condition::eq(Self::Entity::primary_key(), row_id))
            .one(conn)
            .await?;

        row.ok_or(crate::error::Error::NoRowsAffected)
    }

    /// Insert this active model and return only the number of rows affected
    ///
    /// Use this when you don't need the inserted row back (slightly more efficient).
    async fn insert_exec(self, conn: &crate::Connection) -> Result<u64> {
        crate::query::Insert::<Self::Entity>::new(self).exec(conn).await
    }

    /// Update this active model in the database and return the updated row
    ///
    /// The active model must have its primary key set. Only fields that are
    /// `Set` will be updated.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut update = UserEntity::into_active_model(user);
    /// update.name = set("New Name".to_string());
    ///
    /// let updated = update.update(&conn).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error::PrimaryKeyNotSet` if the primary key is not set.
    async fn update(self, conn: &crate::Connection) -> Result<<Self::Entity as EntityTrait>::Model>
    where <Self::Entity as EntityTrait>::Model: Send {
        crate::query::Update::<Self::Entity>::new(self).exec_with_returning(conn).await
    }

    /// Update this active model and return only the number of rows affected
    ///
    /// Use this when you don't need the updated row back.
    async fn update_exec(self, conn: &crate::Connection) -> Result<u64> {
        crate::query::Update::<Self::Entity>::new(self).exec(conn).await
    }

    /// Delete this active model from the database
    ///
    /// The active model must have its primary key set.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let to_delete = UserEntity::into_active_model(user);
    /// let rows_affected = to_delete.delete(&conn).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error::PrimaryKeyNotSet` if the primary key is not set.
    async fn delete(self, conn: &crate::Connection) -> Result<u64> {
        let pk_value = self.get_primary_key_value().ok_or(crate::error::Error::PrimaryKeyNotSet)?;
        crate::query::Delete::<Self::Entity>::new()
            .filter(crate::query::Condition::eq(Self::Entity::primary_key(), pk_value))
            .exec(conn)
            .await
    }
}
