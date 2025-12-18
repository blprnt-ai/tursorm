use crate::error::Result;

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
