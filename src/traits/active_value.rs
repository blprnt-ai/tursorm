/// Active value wrapper for tracking field state in active models
///
/// `ActiveValue` wraps field values in active models to track whether
/// a field has been explicitly set. Only `Set` values are included in
/// INSERT and UPDATE queries.
///
/// # Variants
///
/// - `Set(T)` - The field has a value and will be included in queries
/// - `NotSet` - The field has no value and will be excluded from queries
///
/// # Example
///
/// ```ignore
/// use tursorm::{ActiveValue, set, not_set};
///
/// // Using the set() helper
/// let name: ActiveValue<String> = set("Alice".to_string());
///
/// // Using the not_set() helper
/// let id: ActiveValue<i64> = not_set();
///
/// // Using From trait
/// let age: ActiveValue<i32> = 25.into();
///
/// // Check state
/// assert!(name.is_set());
/// assert!(id.is_not_set());
/// ```
#[derive(Clone, Debug)]
pub enum ActiveValue<T> {
    /// Value is set and will be included in queries
    Set(T),
    /// Value is not set and will be excluded from queries
    NotSet,
}

impl<T> Default for ActiveValue<T> {
    fn default() -> Self {
        ActiveValue::NotSet
    }
}

impl<T> ActiveValue<T> {
    /// Create a new set value
    pub fn set(value: T) -> Self {
        ActiveValue::Set(value)
    }

    /// Check if value is set
    pub fn is_set(&self) -> bool {
        matches!(self, ActiveValue::Set(_))
    }

    /// Check if value is not set
    pub fn is_not_set(&self) -> bool {
        matches!(self, ActiveValue::NotSet)
    }

    /// Get a reference to the value if set
    ///
    /// Returns `None` if the value is `NotSet`.
    pub fn get(&self) -> Option<&T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    /// Take ownership of the value if set, consuming self
    ///
    /// Returns `None` if the value is `NotSet`.
    pub fn take(self) -> Option<T> {
        match self {
            ActiveValue::Set(v) => Some(v),
            ActiveValue::NotSet => None,
        }
    }

    /// Unwrap the value, panicking if not set
    ///
    /// # Panics
    ///
    /// Panics with message "Called unwrap on NotSet ActiveValue" if
    /// the value is `NotSet`.
    pub fn unwrap(self) -> T {
        match self {
            ActiveValue::Set(v) => v,
            ActiveValue::NotSet => panic!("Called unwrap on NotSet ActiveValue"),
        }
    }
}

impl<T> From<T> for ActiveValue<T> {
    fn from(value: T) -> Self {
        ActiveValue::Set(value)
    }
}

/// Create a `Set` active value (shorthand for `ActiveValue::Set`)
///
/// This is the recommended way to set field values in active models.
///
/// # Example
///
/// ```ignore
/// let user = UserActiveModel {
///     name: set("Alice".to_string()),
///     email: set("alice@example.com".to_string()),
///     ..Default::default()
/// };
/// ```
pub fn set<T>(value: T) -> ActiveValue<T> {
    ActiveValue::Set(value)
}

/// Create a `NotSet` active value (shorthand for `ActiveValue::NotSet`)
///
/// Fields with `NotSet` values are excluded from INSERT and UPDATE queries.
///
/// # Example
///
/// ```ignore
/// let id: ActiveValue<i64> = not_set();
/// assert!(id.is_not_set());
/// ```
pub fn not_set<T>() -> ActiveValue<T> {
    ActiveValue::NotSet
}
