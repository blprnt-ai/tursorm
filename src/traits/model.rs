use super::entity::EntityTrait;
use crate::value::Value;

/// Trait for model types that represent database rows
///
/// A model is an immutable struct that holds the data from a database row.
/// It's the result type when querying with SELECT operations.
///
/// # Example
///
/// ```ignore
/// // Query for records
/// let users = User::find()
///     .filter(Condition::eq(UserColumn::Status, "active"))
///     .all(&conn)
///     .await?;
///
/// // Find by ID
/// let user = User::find_by_id(1).one(&conn).await?;
///
/// // Convert to active model for updates or deletes
/// if let Some(user) = user {
///     let mut active = user.into_active_model();
///     active.name = set("Updated".to_string());
///     active.update(&conn).await?;
/// }
/// ```
pub trait ModelTrait: Clone + Send + Sync {
    /// The entity type for this model
    type Entity: EntityTrait;

    /// Get the primary key value
    fn get_primary_key_value(&self) -> Value;

    /// Convert this model into an active model for updates or deletes
    ///
    /// All fields will be set to `Set` with the model's current values.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = User::find_by_id(1).one(&conn).await?.unwrap();
    ///
    /// // Update
    /// let mut active = user.into_active_model();
    /// active.name = set("New Name".to_string());
    /// let updated = active.update(&conn).await?;
    ///
    /// // Delete
    /// let active = user.into_active_model();
    /// active.delete(&conn).await?;
    /// ```
    fn into_active_model(self) -> <Self::Entity as EntityTrait>::ActiveModel
    where <Self::Entity as EntityTrait>::ActiveModel: From<Self> {
        <Self::Entity as EntityTrait>::ActiveModel::from(self)
    }
}
