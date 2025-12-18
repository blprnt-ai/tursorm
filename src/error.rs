//! Error types for tursorm

use thiserror::Error;

/// Result type alias for tursorm operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in tursorm operations
#[derive(Error, Debug)]
pub enum Error {
    /// Database error from turso
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),

    /// Type conversion error
    #[error("Type conversion error: expected {expected}, got {actual}")]
    TypeConversion { expected: &'static str, actual: String },

    /// Null value error when a non-nullable field received null
    #[error("Unexpected null value for non-nullable field")]
    UnexpectedNull,

    /// Column not found
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// No rows affected when expected
    #[error("No rows affected")]
    NoRowsAffected,

    /// Primary key not set for update operation
    #[error("Primary key must be set for update operation")]
    PrimaryKeyNotSet,

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// JSON serialization/deserialization error
    #[cfg(any(feature = "with-json", feature = "with-arrays"))]
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_type_conversion() {
        let err = Error::TypeConversion { expected: "Integer", actual: "Text(hello)".to_string() };
        let display = format!("{}", err);
        assert!(display.contains("Type conversion error"));
        assert!(display.contains("Integer"));
        assert!(display.contains("Text(hello)"));
    }

    #[test]
    fn test_error_display_unexpected_null() {
        let err = Error::UnexpectedNull;
        let display = format!("{}", err);
        assert!(display.contains("Unexpected null"));
    }

    #[test]
    fn test_error_display_column_not_found() {
        let err = Error::ColumnNotFound("user_id".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Column not found"));
        assert!(display.contains("user_id"));
    }

    #[test]
    fn test_error_display_no_rows_affected() {
        let err = Error::NoRowsAffected;
        let display = format!("{}", err);
        assert!(display.contains("No rows affected"));
    }

    #[test]
    fn test_error_display_primary_key_not_set() {
        let err = Error::PrimaryKeyNotSet;
        let display = format!("{}", err);
        assert!(display.contains("Primary key must be set"));
    }

    #[test]
    fn test_error_display_query() {
        let err = Error::Query("Invalid SQL syntax".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Query error"));
        assert!(display.contains("Invalid SQL syntax"));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::UnexpectedNull;
        let debug = format!("{:?}", err);
        assert!(debug.contains("UnexpectedNull"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result type alias works correctly
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(Error::UnexpectedNull)
        }

        assert!(returns_ok().is_ok());
        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_type_conversion_variants() {
        // Test various type conversion error scenarios
        let err1 = Error::TypeConversion { expected: "Integer", actual: "Text".to_string() };
        let err2 = Error::TypeConversion { expected: "Real", actual: "Blob".to_string() };

        assert!(format!("{}", err1).contains("Integer"));
        assert!(format!("{}", err2).contains("Real"));
    }

    #[test]
    fn test_error_query_empty_message() {
        let err = Error::Query(String::new());
        let display = format!("{}", err);
        assert!(display.contains("Query error"));
    }

    #[test]
    fn test_error_column_not_found_empty() {
        let err = Error::ColumnNotFound(String::new());
        let display = format!("{}", err);
        assert!(display.contains("Column not found"));
    }
}
