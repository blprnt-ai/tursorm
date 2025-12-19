pub(crate) mod active_model;
pub(crate) mod active_value;
pub(crate) mod column;
pub(crate) mod entity;
pub(crate) mod from_row;
pub(crate) mod model;

pub mod prelude {
    pub use super::active_model::ActiveModelTrait;
    pub use super::active_value::ActiveValue;
    pub use super::active_value::not_set;
    pub use super::active_value::set;
    pub use super::column::ColumnTrait;
    pub use super::column::ForeignKeyInfo;
    pub use super::column::OnDelete;
    pub use super::column::OnUpdate;
    pub use super::entity::EntityTrait;
    pub use super::from_row::FromRow;
    pub use super::model::ModelTrait;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

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
            ActiveValue::NotSet => {}
        }
    }
}
