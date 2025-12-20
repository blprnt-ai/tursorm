#[derive(Clone, Debug)]
pub enum FieldValue<T: PartialEq> {
    Set(T),
    NotSet,
}

impl<V: PartialEq> PartialEq for FieldValue<V> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldValue::Set(a), FieldValue::Set(b)) => a == b,
            (FieldValue::NotSet, FieldValue::NotSet) => true,
            _ => false,
        }
    }
}

impl<V: PartialEq> Default for FieldValue<V> {
    fn default() -> Self {
        FieldValue::NotSet
    }
}

impl<V: PartialEq> FieldValue<V> {
    pub fn set(value: V) -> Self {
        FieldValue::Set(value)
    }

    pub fn is_changed(&self) -> bool {
        matches!(self, FieldValue::Set(_))
    }

    pub fn is_not_set(&self) -> bool {
        matches!(self, FieldValue::NotSet)
    }

    pub fn get(&self) -> Option<&V> {
        match self {
            FieldValue::Set(v) => Some(v),
            FieldValue::NotSet => None,
        }
    }

    pub fn take(self) -> Option<V> {
        match self {
            FieldValue::Set(v) => Some(v),
            FieldValue::NotSet => None,
        }
    }

    pub fn unwrap(self) -> V {
        match self {
            FieldValue::Set(v) => v,
            FieldValue::NotSet => panic!("Called unwrap on NotSet FieldValue"),
        }
    }
}

impl<V: PartialEq> From<V> for FieldValue<V> {
    fn from(value: V) -> Self {
        FieldValue::Set(value)
    }
}

pub fn set<V: PartialEq>(value: V) -> FieldValue<V> {
    FieldValue::Set(value)
}

pub fn not_set<V: PartialEq>() -> FieldValue<V> {
    FieldValue::NotSet
}
