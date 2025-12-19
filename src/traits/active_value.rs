#[derive(Clone, Debug)]
pub enum ActiveValue<T: PartialEq> {
    Set(T),

    NotSet,
}

impl<T: PartialEq> PartialEq for ActiveValue<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ActiveValue::Set(a), ActiveValue::Set(b)) => a == b,
            (ActiveValue::NotSet, ActiveValue::NotSet) => true,
            _ => false,
        }
    }
}

impl<T: PartialEq> Default for ActiveValue<T> {
    fn default() -> Self {
        ActiveValue::NotSet
    }
}

impl<T: PartialEq> ActiveValue<T> {
    pub fn set(value: T) -> Self {
        ActiveValue::Set(value)
    }

    pub fn is_set(&self) -> bool {
        matches!(self, ActiveValue::Set(_))
    }

    pub fn is_not_set(&self) -> bool {
        matches!(self, ActiveValue::NotSet)
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    pub fn take(self) -> Option<T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            ActiveValue::Set(v) => v,
            ActiveValue::NotSet => panic!("Called unwrap on NotSet ActiveValue"),
        }
    }
}

impl<T: PartialEq> From<T> for ActiveValue<T> {
    fn from(value: T) -> Self {
        ActiveValue::Set(value)
    }
}

pub fn set<T: PartialEq>(value: T) -> ActiveValue<T> {
    ActiveValue::Set(value)
}

pub fn not_set<T: PartialEq>() -> ActiveValue<T> {
    ActiveValue::NotSet
}
