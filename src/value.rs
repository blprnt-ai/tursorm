pub use turso::Value;

use crate::error::Error;
use crate::error::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Blob,
    Null,
}

pub trait IntoValue: std::fmt::Debug {
    fn into_value(self) -> Value;
}

pub trait FromValue: std::fmt::Debug + Sized {
    fn from_value(value: Value) -> Result<Self>;

    fn from_value_opt(value: Value) -> Result<Self>
    where Self: Default {
        if matches!(value, Value::Null) { Ok(Self::default()) } else { Self::from_value(value) }
    }
}

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

impl IntoValue for u64 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for isize {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

impl IntoValue for usize {
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

impl<V: IntoValue> IntoValue for Option<V> {
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

impl FromValue for u64 {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as u64)
    }
}

impl FromValue for isize {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as isize)
    }
}

impl FromValue for usize {
    fn from_value(value: Value) -> Result<Self> {
        i64::from_value(value).map(|v| v as usize)
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

impl<V: FromValue> FromValue for Option<V> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            other => V::from_value(other).map(Some),
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

#[cfg(feature = "with-json")]
pub use json_impl::Json;

#[cfg(feature = "with-json")]
mod json_impl {
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use serde_json::Value as JsonValue;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Json<T>(pub T);

    impl<V: Serialize + std::fmt::Debug> IntoValue for Json<V> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self.0) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl<V: DeserializeOwned + std::fmt::Debug> FromValue for Json<V> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: V = serde_json::from_str(&s)?;
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

#[cfg(feature = "with-arrays")]
mod arrays_impl {
    use super::*;

    impl IntoValue for Vec<String> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<String> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<String> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for Vec<i64> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<i64> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<i64> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for Vec<i32> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<i32> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<i32> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for Vec<f64> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<f64> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<f64> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for Vec<f32> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<f32> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<f32> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }

    impl IntoValue for Vec<bool> {
        fn into_value(self) -> Value {
            match serde_json::to_string(&self) {
                Ok(s) => Value::Text(s),
                Err(_) => Value::Null,
            }
        }
    }

    impl FromValue for Vec<bool> {
        fn from_value(value: Value) -> Result<Self> {
            match value {
                Value::Text(s) => {
                    let parsed: Vec<bool> = serde_json::from_str(&s)?;
                    Ok(parsed)
                }
                Value::Null => Err(Error::UnexpectedNull),
                other => Err(Error::TypeConversion { expected: "Text (JSON array)", actual: format!("{:?}", other) }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_bool_into_value() {
        assert_eq!(true.into_value(), Value::Integer(1));
        assert_eq!(false.into_value(), Value::Integer(0));
    }

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

    #[test]
    fn test_value_into_value() {
        let val = Value::Integer(42);
        assert_eq!(val.clone().into_value(), val);
    }

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

    #[test]
    fn test_value_from_value() {
        let val = Value::Integer(42);
        assert_eq!(Value::from_value(val.clone()).unwrap(), val);
    }

    #[test]
    fn test_from_value_opt_with_value() {
        let val = Value::Integer(42);
        assert_eq!(i64::from_value_opt(val).unwrap(), 42);
    }

    #[test]
    fn test_from_value_opt_with_null() {
        let val = Value::Null;
        assert_eq!(i64::from_value_opt(val).unwrap(), 0);
    }

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
        let val = f64::INFINITY;
        assert_eq!(val.into_value(), Value::Real(f64::INFINITY));

        let val = f64::NEG_INFINITY;
        assert_eq!(val.into_value(), Value::Real(f64::NEG_INFINITY));
    }

    #[cfg(feature = "with-arrays")]
    mod vec_tests {
        use super::*;

        #[test]
        fn test_vec_string_into_value() {
            let val = vec!["hello".to_string(), "world".to_string()];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[\"hello\",\"world\"]".to_string()));
        }

        #[test]
        fn test_vec_string_into_value_empty() {
            let val: Vec<String> = vec![];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[]".to_string()));
        }

        #[test]
        fn test_vec_string_from_value() {
            let val = Value::Text("[\"hello\",\"world\"]".to_string());
            let result: Vec<String> = Vec::<String>::from_value(val).unwrap();
            assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
        }

        #[test]
        fn test_vec_string_from_value_empty() {
            let val = Value::Text("[]".to_string());
            let result: Vec<String> = Vec::<String>::from_value(val).unwrap();
            assert!(result.is_empty());
        }

        #[test]
        fn test_vec_string_from_null() {
            let val = Value::Null;
            assert!(Vec::<String>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_string_from_invalid_type() {
            let val = Value::Integer(42);
            assert!(Vec::<String>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_i64_into_value() {
            let val = vec![1i64, 2, 3, 4, 5];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[1,2,3,4,5]".to_string()));
        }

        #[test]
        fn test_vec_i64_from_value() {
            let val = Value::Text("[1,2,3,4,5]".to_string());
            let result: Vec<i64> = Vec::<i64>::from_value(val).unwrap();
            assert_eq!(result, vec![1i64, 2, 3, 4, 5]);
        }

        #[test]
        fn test_vec_i64_from_null() {
            let val = Value::Null;
            assert!(Vec::<i64>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_i64_negative_values() {
            let val = vec![-1i64, -2, -3];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[-1,-2,-3]".to_string()));

            let parsed: Vec<i64> = Vec::<i64>::from_value(result).unwrap();
            assert_eq!(parsed, vec![-1i64, -2, -3]);
        }

        #[test]
        fn test_vec_i32_into_value() {
            let val = vec![1i32, 2, 3];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[1,2,3]".to_string()));
        }

        #[test]
        fn test_vec_i32_from_value() {
            let val = Value::Text("[1,2,3]".to_string());
            let result: Vec<i32> = Vec::<i32>::from_value(val).unwrap();
            assert_eq!(result, vec![1i32, 2, 3]);
        }

        #[test]
        fn test_vec_i32_from_null() {
            let val = Value::Null;
            assert!(Vec::<i32>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_f64_into_value() {
            let val = vec![1.5f64, 2.5, 3.5];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[1.5,2.5,3.5]".to_string()));
        }

        #[test]
        fn test_vec_f64_from_value() {
            let val = Value::Text("[1.5,2.5,3.5]".to_string());
            let result: Vec<f64> = Vec::<f64>::from_value(val).unwrap();
            assert_eq!(result, vec![1.5f64, 2.5, 3.5]);
        }

        #[test]
        fn test_vec_f64_from_null() {
            let val = Value::Null;
            assert!(Vec::<f64>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_f32_into_value() {
            let val = vec![1.5f32, 2.5, 3.5];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[1.5,2.5,3.5]".to_string()));
        }

        #[test]
        fn test_vec_f32_from_value() {
            let val = Value::Text("[1.5,2.5,3.5]".to_string());
            let result: Vec<f32> = Vec::<f32>::from_value(val).unwrap();
            assert_eq!(result, vec![1.5f32, 2.5, 3.5]);
        }

        #[test]
        fn test_vec_f32_from_null() {
            let val = Value::Null;
            assert!(Vec::<f32>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_bool_into_value() {
            let val = vec![true, false, true];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[true,false,true]".to_string()));
        }

        #[test]
        fn test_vec_bool_from_value() {
            let val = Value::Text("[true,false,true]".to_string());
            let result: Vec<bool> = Vec::<bool>::from_value(val).unwrap();
            assert_eq!(result, vec![true, false, true]);
        }

        #[test]
        fn test_vec_bool_from_null() {
            let val = Value::Null;
            assert!(Vec::<bool>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_bool_empty() {
            let val: Vec<bool> = vec![];
            let result = val.into_value();
            assert_eq!(result, Value::Text("[]".to_string()));

            let parsed: Vec<bool> = Vec::<bool>::from_value(result).unwrap();
            assert!(parsed.is_empty());
        }

        #[test]
        fn test_vec_string_roundtrip() {
            let original = vec!["a".to_string(), "b".to_string(), "c".to_string()];
            let value = original.clone().into_value();
            let parsed: Vec<String> = Vec::<String>::from_value(value).unwrap();
            assert_eq!(original, parsed);
        }

        #[test]
        fn test_vec_i64_roundtrip() {
            let original = vec![i64::MIN, 0, i64::MAX];
            let value = original.clone().into_value();
            let parsed: Vec<i64> = Vec::<i64>::from_value(value).unwrap();
            assert_eq!(original, parsed);
        }

        #[test]
        fn test_vec_f64_roundtrip() {
            let original = vec![0.0, 1.0, -1.0, 123.456];
            let value = original.clone().into_value();
            let parsed: Vec<f64> = Vec::<f64>::from_value(value).unwrap();
            assert_eq!(original, parsed);
        }

        #[test]
        fn test_vec_string_from_invalid_json() {
            let val = Value::Text("not valid json".to_string());
            assert!(Vec::<String>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_i64_from_invalid_json() {
            let val = Value::Text("[\"not\", \"numbers\"]".to_string());
            assert!(Vec::<i64>::from_value(val).is_err());
        }

        #[test]
        fn test_vec_bool_from_invalid_json() {
            let val = Value::Text("[1, 0]".to_string());
            assert!(Vec::<bool>::from_value(val).is_err());
        }
    }
}
