//! Value types and conversions for tursorm

pub use turso::Value;

use crate::error::Error;
use crate::error::Result;

/// Column types supported by the ORM
///
/// These types map to SQLite's type affinity system:
/// - `Integer` maps to INTEGER (64-bit signed)
/// - `Float` maps to REAL (64-bit floating point)
/// - `Text` maps to TEXT (UTF-8 string)
/// - `Blob` maps to BLOB (binary data)
/// - `Null` maps to NULL
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    /// 64-bit signed integer (SQLite INTEGER)
    Integer,
    /// 64-bit floating point number (SQLite REAL)
    Float,
    /// UTF-8 text string (SQLite TEXT)
    Text,
    /// Binary data (SQLite BLOB)
    Blob,
    /// Null value (SQLite NULL)
    Null,
}

/// Trait for converting Rust types into database values
///
/// This trait is implemented for common Rust types to allow them to be used
/// as query parameters. Custom types can implement this trait to be used
/// with the ORM.
///
/// # Example
///
/// ```ignore
/// use tursorm::IntoValue;
///
/// let value: Value = 42i64.into_value();
/// let text: Value = "hello".into_value();
/// ```
pub trait IntoValue {
    /// Convert this value into a database [`Value`]
    fn into_value(self) -> Value;
}

/// Trait for converting database values into Rust types
///
/// This trait is implemented for common Rust types to allow them to be
/// extracted from database query results. Custom types can implement this
/// trait to be used with the ORM.
///
/// # Example
///
/// ```ignore
/// use tursorm::{FromValue, Value};
///
/// let value = Value::Integer(42);
/// let num: i64 = i64::from_value(value)?;
/// ```
pub trait FromValue: Sized {
    /// Convert a database [`Value`] into this type
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to this type,
    /// or if the value is null and this type is not nullable.
    fn from_value(value: Value) -> Result<Self>;

    /// Convert from value, returning the default value for null
    ///
    /// This is useful for nullable columns where you want to use a default
    /// value instead of `Option<T>`.
    fn from_value_opt(value: Value) -> Result<Self>
    where Self: Default {
        if matches!(value, Value::Null) { Ok(Self::default()) } else { Self::from_value(value) }
    }
}

// Implement IntoValue for common types

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::Integer(self)
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for i16 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for i8 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for u32 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for u16 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for u8 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::Real(self)
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::Real(self as f64)
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Text(self)
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Text(self.to_string())
    }
}

impl IntoValue for Vec<u8> {
    fn into_value(self) -> Value {
        Value::Blob(self)
    }
}

impl IntoValue for &[u8] {
    fn into_value(self) -> Value {
        Value::Blob(self.to_vec())
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Integer(if self { 1 } else { 0 })
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::Null,
        }
    }
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

// Implement FromValue for common types

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(v) => Ok(v),
            Value::Real(v) => Ok(v as i64),
            Value::Null => Err(Error::UnexpectedNull),
            other => Err(Error::TypeConversion { expected: "Integer", actual: format!("{:?}", other) }),
        }
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as i32)
    }
}

impl FromValue for i16 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as i16)
    }
}

impl FromValue for i8 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as i8)
    }
}

impl FromValue for u32 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as u32)
    }
}

impl FromValue for u16 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as u16)
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as u8)
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Real(v) => Ok(v),
            Value::Integer(v) => Ok(v as f64),
            Value::Null => Err(Error::UnexpectedNull),
            other => Err(Error::TypeConversion { expected: "Real", actual: format!("{:?}", other) }),
        }
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self> {
        f64::from_value(value).map(|v| v as f32)
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Text(v) => Ok(v),
            Value::Null => Err(Error::UnexpectedNull),
            other => Err(Error::TypeConversion { expected: "Text", actual: format!("{:?}", other) }),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Blob(v) => Ok(v),
            Value::Null => Err(Error::UnexpectedNull),
            other => Err(Error::TypeConversion { expected: "Blob", actual: format!("{:?}", other) }),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(v) => Ok(v != 0),
            Value::Null => Err(Error::UnexpectedNull),
            other => Err(Error::TypeConversion { expected: "Integer (boolean)", actual: format!("{:?}", other) }),
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            other => T::from_value(other).map(Some),
        }
    }

    fn from_value_opt(value: Value) -> Result<Self> {
        Self::from_value(value)
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self> {
        Ok(value)
    }
}

// Optional chrono support
#[cfg(feature = "with-chrono")]
mod chrono_impl {
    use chrono::DateTime;
    use chrono::NaiveDate;
    use chrono::NaiveDateTime;
    use chrono::NaiveTime;
    use chrono::Utc;

    use super::*;

    impl IntoValue for NaiveDateTime {
        fn into_value(self) -> Value {
            Value::Text(self.format("%Y-%m-%d %H:%M:%S").to_string())
        }
    }

    impl FromValue for NaiveDateTime {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S"))
                    .map_err(|_| Error::TypeConversion { expected: "NaiveDateTime", actual: s }),
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (datetime)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for DateTime<Utc> {
        fn into_value(self) -> Value {
            Value::Text(self.format("%Y-%m-%d %H:%M:%S").to_string())
        }
    }

    impl FromValue for DateTime<Utc> {
        fn from_value(value: Value) -> Result<Self> {
            let ndt = NaiveDateTime::from_value(value)?;
            Ok(DateTime::from_naive_utc_and_offset(ndt, Utc))
        }
    }

    impl IntoValue for NaiveDate {
        fn into_value(self) -> Value {
            Value::Text(self.format("%Y-%m-%d").to_string())
        }
    }

    impl FromValue for NaiveDate {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                    .map_err(|_| Error::TypeConversion { expected: "NaiveDate", actual: s }),
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (date)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for NaiveTime {
        fn into_value(self) -> Value {
            Value::Text(self.format("%H:%M:%S").to_string())
        }
    }

    impl FromValue for NaiveTime {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => NaiveTime::parse_from_str(&s, "%H:%M:%S")
                    .map_err(|_| Error::TypeConversion { expected: "NaiveTime", actual: s }),
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (time)", actual: format!("{:?}", other) }),
            }
        }
    }
}

// Optional UUID support
#[cfg(feature = "with-uuid")]
mod uuid_impl {
    use uuid::Uuid;

    use super::*;

    impl IntoValue for Uuid {
        fn into_value(self) -> Value {
            Value::Text(self.to_string())
        }
    }

    impl FromValue for Uuid {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    Uuid::parse_str(&s).map_err(|_| Error::TypeConversion { expected: "UUID", actual: s })
                }
                Value::Blob(b) => Uuid::from_slice(&b)
                    .map_err(|_| Error::TypeConversion { expected: "UUID", actual: format!("{:?}", b) }),
                Value::Null => Err(Error::UnexpectedNull),
                other => {
                    Err(Error::TypeConversion { expected: "Text or Blob (UUID)", actual: format!("{:?}", other) })
                }
            }
        }
    }
}

// Optional JSON support
#[cfg(feature = "with-json")]
pub use json_impl::Json;

#[cfg(feature = "with-json")]
mod json_impl {
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use serde_json::Value as JsonValue;

    use super::*;

    /// Wrapper type for JSON values
    #[derive(Clone, Debug, PartialEq)]
    pub struct Json<T>(pub T);

    impl<T: Serialize> IntoValue for Json<T> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self.0) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl<T: DeserializeOwned> FromValue for Json<T> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: T = serde_json::from_str(&s)?;
                    Ok(Json(parsed))
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for JsonValue {
        fn into_value(self) -> Value {
            Value::Text(self.to_string())
        }
    }

    impl FromValue for JsonValue {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: JsonValue = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Ok(JsonValue::Null),
                other => Err(Error::TypeConversion { expected: "Text (JSON)", actual: format!("{:?}", other) }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ColumnType tests
    #[test]
    fn test_column_type_equality() {
        assert_eq!(ColumnType::Integer, ColumnType::Integer);
        assert_eq!(ColumnType::Float, ColumnType::Float);
        assert_eq!(ColumnType::Text, ColumnType::Text);
        assert_eq!(ColumnType::Blob, ColumnType::Blob);
        assert_eq!(ColumnType::Null, ColumnType::Null);
        assert_ne!(ColumnType::Integer, ColumnType::Float);
    }

    #[test]
    fn test_column_type_clone() {
        let ct = ColumnType::Integer;
        let cloned = ct.clone();
        assert_eq!(ct, cloned);
    }

    #[test]
    fn test_column_type_debug() {
        assert_eq!(format!("{:?}", ColumnType::Integer), "Integer");
        assert_eq!(format!("{:?}", ColumnType::Float), "Float");
        assert_eq!(format!("{:?}", ColumnType::Text), "Text");
        assert_eq!(format!("{:?}", ColumnType::Blob), "Blob");
        assert_eq!(format!("{:?}", ColumnType::Null), "Null");
    }

    // IntoValue tests for integer types
    #[test]
    fn test_i64_into_value() {
        let val: i64 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_i32_into_value() {
        let val: i32 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_i16_into_value() {
        let val: i16 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_i8_into_value() {
        let val: i8 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_u32_into_value() {
        let val: u32 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_u16_into_value() {
        let val: u16 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_u8_into_value() {
        let val: u8 = 42;
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    // IntoValue tests for float types
    #[test]
    fn test_f64_into_value() {
        let val: f64 = 3.14;
        assert_eq!(val.into_value(), Value::Real(3.14));
    }

    #[test]
    fn test_f32_into_value() {
        let val: f32 = 3.14;
        let result = val.into_value();
        match result {
            Value::Real(v) => assert!((v - 3.14).abs() < 0.001),
            _ => panic!("Expected Real value"),
        }
    }

    // IntoValue tests for string types
    #[test]
    fn test_string_into_value() {
        let val = String::from("hello");
        assert_eq!(val.into_value(), Value::Text("hello".to_string()));
    }

    #[test]
    fn test_str_into_value() {
        let val = "hello";
        assert_eq!(val.into_value(), Value::Text("hello".to_string()));
    }

    // IntoValue tests for blob types
    #[test]
    fn test_vec_u8_into_value() {
        let val: Vec<u8> = vec![1, 2, 3];
        assert_eq!(val.into_value(), Value::Blob(vec![1, 2, 3]));
    }

    #[test]
    fn test_slice_u8_into_value() {
        let val: &[u8] = &[1, 2, 3];
        assert_eq!(val.into_value(), Value::Blob(vec![1, 2, 3]));
    }

    // IntoValue tests for bool
    #[test]
    fn test_bool_into_value() {
        assert_eq!(true.into_value(), Value::Integer(1));
        assert_eq!(false.into_value(), Value::Integer(0));
    }

    // IntoValue tests for Option
    #[test]
    fn test_option_some_into_value() {
        let val: Option<i64> = Some(42);
        assert_eq!(val.into_value(), Value::Integer(42));
    }

    #[test]
    fn test_option_none_into_value() {
        let val: Option<i64> = None;
        assert_eq!(val.into_value(), Value::Null);
    }

    // IntoValue for Value (identity)
    #[test]
    fn test_value_into_value() {
        let val = Value::Integer(42);
        assert_eq!(val.clone().into_value(), val);
    }

    // FromValue tests for integer types
    #[test]
    fn test_i64_from_value() {
        let val = Value::Integer(42);
        assert_eq!(i64::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_i64_from_real_value() {
        let val = Value::Real(42.7);
        assert_eq!(i64::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_i64_from_null_value() {
        let val = Value::Null;
        assert!(i64::from_value(val).is_err());
    }

    #[test]
    fn test_i64_from_invalid_type() {
        let val = Value::Text("hello".to_string());
        assert!(i64::from_value(val).is_err());
    }

    #[test]
    fn test_i32_from_value() {
        let val = Value::Integer(42);
        assert_eq!(i32::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_i16_from_value() {
        let val = Value::Integer(42);
        assert_eq!(i16::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_i8_from_value() {
        let val = Value::Integer(42);
        assert_eq!(i8::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_u32_from_value() {
        let val = Value::Integer(42);
        assert_eq!(u32::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_u16_from_value() {
        let val = Value::Integer(42);
        assert_eq!(u16::from_value(val).unwrap(), 42);
    }

    #[test]
    fn test_u8_from_value() {
        let val = Value::Integer(42);
        assert_eq!(u8::from_value(val).unwrap(), 42);
    }

    // FromValue tests for float types
    #[test]
    fn test_f64_from_value() {
        let val = Value::Real(3.14);
        assert!((f64::from_value(val).unwrap() - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_f64_from_integer_value() {
        let val = Value::Integer(42);
        assert!((f64::from_value(val).unwrap() - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_f64_from_null_value() {
        let val = Value::Null;
        assert!(f64::from_value(val).is_err());
    }

    #[test]
    fn test_f32_from_value() {
        let val = Value::Real(3.14);
        assert!((f32::from_value(val).unwrap() - 3.14).abs() < 0.01);
    }

    // FromValue tests for String
    #[test]
    fn test_string_from_value() {
        let val = Value::Text("hello".to_string());
        assert_eq!(String::from_value(val).unwrap(), "hello");
    }

    #[test]
    fn test_string_from_null_value() {
        let val = Value::Null;
        assert!(String::from_value(val).is_err());
    }

    #[test]
    fn test_string_from_invalid_type() {
        let val = Value::Integer(42);
        assert!(String::from_value(val).is_err());
    }

    // FromValue tests for Vec<u8>
    #[test]
    fn test_vec_u8_from_value() {
        let val = Value::Blob(vec![1, 2, 3]);
        assert_eq!(Vec::<u8>::from_value(val).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_vec_u8_from_null_value() {
        let val = Value::Null;
        assert!(Vec::<u8>::from_value(val).is_err());
    }

    // FromValue tests for bool
    #[test]
    fn test_bool_from_value() {
        assert!(bool::from_value(Value::Integer(1)).unwrap());
        assert!(!bool::from_value(Value::Integer(0)).unwrap());
        assert!(bool::from_value(Value::Integer(42)).unwrap());
    }

    #[test]
    fn test_bool_from_null_value() {
        let val = Value::Null;
        assert!(bool::from_value(val).is_err());
    }

    // FromValue tests for Option
    #[test]
    fn test_option_from_value_some() {
        let val = Value::Integer(42);
        assert_eq!(Option::<i64>::from_value(val).unwrap(), Some(42));
    }

    #[test]
    fn test_option_from_value_null() {
        let val = Value::Null;
        assert_eq!(Option::<i64>::from_value(val).unwrap(), None);
    }

    // FromValue for Value (identity)
    #[test]
    fn test_value_from_value() {
        let val = Value::Integer(42);
        assert_eq!(Value::from_value(val.clone()).unwrap(), val);
    }

    // from_value_opt tests
    #[test]
    fn test_from_value_opt_with_value() {
        let val = Value::Integer(42);
        assert_eq!(i64::from_value_opt(val).unwrap(), 42);
    }

    #[test]
    fn test_from_value_opt_with_null() {
        let val = Value::Null;
        assert_eq!(i64::from_value_opt(val).unwrap(), 0); // Default for i64
    }

    // Edge cases
    #[test]
    fn test_negative_integers() {
        let val: i64 = -42;
        assert_eq!(val.into_value(), Value::Integer(-42));
        assert_eq!(i64::from_value(Value::Integer(-42)).unwrap(), -42);
    }

    #[test]
    fn test_large_integers() {
        let val: i64 = i64::MAX;
        assert_eq!(val.into_value(), Value::Integer(i64::MAX));
        assert_eq!(i64::from_value(Value::Integer(i64::MAX)).unwrap(), i64::MAX);
    }

    #[test]
    fn test_empty_string() {
        let val = String::new();
        assert_eq!(val.into_value(), Value::Text(String::new()));
        assert_eq!(String::from_value(Value::Text(String::new())).unwrap(), "");
    }

    #[test]
    fn test_empty_blob() {
        let val: Vec<u8> = Vec::new();
        assert_eq!(val.into_value(), Value::Blob(Vec::new()));
        assert_eq!(Vec::<u8>::from_value(Value::Blob(Vec::new())).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_special_float_values() {
        // Test positive infinity
        let val = f64::INFINITY;
        assert_eq!(val.into_value(), Value::Real(f64::INFINITY));

        // Test negative infinity
        let val = f64::NEG_INFINITY;
        assert_eq!(val.into_value(), Value::Real(f64::NEG_INFINITY));
    }
}
