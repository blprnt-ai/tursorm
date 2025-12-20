pub(crate) mod change_set;
pub(crate) mod column;
pub(crate) mod field_value;
pub(crate) mod from_row;
pub(crate) mod record;
pub(crate) mod table;

pub mod prelude {
    pub use super::change_set::ChangeSetTrait;
    pub use super::column::ColumnTrait;
    pub use super::column::ForeignKeyInfo;
    pub use super::column::OnDelete;
    pub use super::column::OnUpdate;
    pub use super::field_value::FieldValue;
    pub use super::field_value::not_set;
    pub use super::field_value::set;
    pub use super::from_row::FromRow;
    pub use super::record::RecordTrait;
    pub use super::table::TableTrait;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn test_change_set_set() {
        let val = FieldValue::Set(42);
        assert!(val.is_changed());
        assert!(!val.is_not_set());
    }

    #[test]
    fn test_change_set_not_set() {
        let val: FieldValue<i32> = FieldValue::NotSet;
        assert!(!val.is_changed());
        assert!(val.is_not_set());
    }

    #[test]
    fn test_change_set_default() {
        let val: FieldValue<i32> = FieldValue::default();
        assert!(val.is_not_set());
    }

    #[test]
    fn test_change_set_set_fn() {
        let val = FieldValue::<i32>::set(42);
        assert!(val.is_changed());
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_change_set_get_some() {
        let val = FieldValue::Set(42);
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_change_set_get_none() {
        let val: FieldValue<i32> = FieldValue::NotSet;
        assert_eq!(val.get(), None);
    }

    #[test]
    fn test_change_set_take_some() {
        let val = FieldValue::Set(42);
        assert_eq!(val.take(), Some(42));
    }

    #[test]
    fn test_change_set_take_none() {
        let val: FieldValue<i32> = FieldValue::NotSet;
        assert_eq!(val.take(), None);
    }

    #[test]
    fn test_change_set_unwrap_success() {
        let val = FieldValue::Set(42);
        assert_eq!(val.unwrap(), 42);
    }

    #[test]
    #[should_panic(expected = "Called unwrap on NotSet FieldValue")]
    fn test_change_set_unwrap_panic() {
        let val: FieldValue<i32> = FieldValue::NotSet;
        val.unwrap();
    }

    #[test]
    fn test_change_set_from() {
        let val: FieldValue<i32> = 42.into();
        assert!(val.is_changed());
        assert_eq!(val.get(), Some(&42));
    }

    #[test]
    fn test_set_helper() {
        let val = set(42);
        assert!(val.is_changed());
        assert_eq!(val.unwrap(), 42);
    }

    #[test]
    fn test_not_set_helper() {
        let val: FieldValue<i32> = not_set();
        assert!(val.is_not_set());
    }

    #[test]
    fn test_change_set_clone() {
        let val = FieldValue::Set(42);
        let cloned = val.clone();
        assert!(cloned.is_changed());
        assert_eq!(cloned.get(), Some(&42));
    }

    #[test]
    fn test_change_set_debug() {
        let set_val = FieldValue::Set(42);
        let not_set_val: FieldValue<i32> = FieldValue::NotSet;

        assert!(format!("{:?}", set_val).contains("Set(42)"));
        assert!(format!("{:?}", not_set_val).contains("NotSet"));
    }

    #[test]
    fn test_change_set_with_string() {
        let val = set(String::from("hello"));
        assert!(val.is_changed());
        assert_eq!(val.get(), Some(&String::from("hello")));
    }

    #[test]
    fn test_change_set_with_vec() {
        let val = set(vec![1, 2, 3]);
        assert!(val.is_changed());
        assert_eq!(val.get(), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_change_set_with_option() {
        let val = set(Some(42));
        assert!(val.is_changed());
        assert_eq!(val.get(), Some(&Some(42)));
    }

    #[test]
    fn test_change_set_get_then_use() {
        let val = set(42);
        if let Some(v) = val.get() {
            assert_eq!(*v, 42);
        } else {
            panic!("Expected Some value");
        }
    }

    #[test]
    fn test_change_set_pattern_matching() {
        let val = set(42);
        match val {
            FieldValue::Set(v) => assert_eq!(v, 42),
            FieldValue::NotSet => panic!("Expected Set value"),
        }
    }

    #[test]
    fn test_not_set_pattern_matching() {
        let val: FieldValue<i32> = not_set();
        match val {
            FieldValue::Set(_) => panic!("Expected NotSet value"),
            FieldValue::NotSet => {}
        }
    }
}
