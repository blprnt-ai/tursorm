#[derive(Clone, Debug)]
pub enum FieldValue<T: PartialEq> {
    Change(T),
    Keep,
}

impl<T: PartialEq> PartialEq for FieldValue<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldValue::Change(a), FieldValue::Change(b)) => a == b,
            (FieldValue::Keep, FieldValue::Keep) => true,
            _ => false,
        }
    }
}

impl<T: PartialEq> Default for FieldValue<T> {
    fn default() -> Self {
        FieldValue::Keep
    }
}

impl<T: PartialEq> FieldValue<T> {
    pub fn change(value: T) -> Self {
        FieldValue::Change(value)
    }

    pub fn is_changed(&self) -> bool {
        matches!(self, FieldValue::Change(_))
    }

    pub fn is_keep(&self) -> bool {
        matches!(self, FieldValue::Keep)
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            FieldValue::Change(v) => Some(v),
            FieldValue::Keep => None,
        }
    }

    pub fn take(self) -> Option<T> {
        match self {
            FieldValue::Change(v) => Some(v),
            FieldValue::Keep => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            FieldValue::Change(v) => v,
            FieldValue::Keep => panic!("Called unwrap on Keep FieldValue"),
        }
    }
}

impl<T: PartialEq> From<T> for FieldValue<T> {
    fn from(value: T) -> Self {
        FieldValue::Change(value)
    }
}

pub fn change<T: PartialEq>(value: T) -> FieldValue<T> {
    FieldValue::Change(value)
}

pub fn keep<T: PartialEq>() -> FieldValue<T> {
    FieldValue::Keep
}
