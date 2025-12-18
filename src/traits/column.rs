use crate::value::ColumnType;

#[derive(Debug, Clone, Copy, Default)]
pub enum OnDelete {
    Restrict,
    #[default]
    Cascade,
    SetNull,
    SetDefault,
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum OnUpdate {
    Restrict,
    #[default]
    Cascade,
    SetNull,
    SetDefault,
    None,
}

#[derive(Debug, Clone, Default)]
pub struct ForeignKeyInfo {
    pub table_name:  String,
    pub column_name: String,
    pub on_delete:   OnDelete,
    pub on_update:   OnUpdate,
}

/// Trait for column enum types that describe table columns
///
/// Each entity has an associated column enum that implements this trait.
/// The enum variants represent individual columns and provide metadata
/// like column name, type, and constraints.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Copy, Debug)]
/// pub enum UserColumn {
///     Id,
///     Name,
///     Email,
/// }
///
/// impl ColumnTrait for UserColumn {
///     fn name(&self) -> &'static str {
///         match self {
///             UserColumn::Id => "id",
///             UserColumn::Name => "name",
///             UserColumn::Email => "email",
///         }
///     }
///     // ... other methods
/// }
/// ```
pub trait ColumnTrait: Copy + Clone + std::fmt::Debug + std::fmt::Display + 'static {
    /// Get the column name
    fn name(&self) -> &'static str;

    /// Get the column type
    fn column_type(&self) -> ColumnType;

    /// Check if this column is nullable
    fn is_nullable(&self) -> bool {
        false
    }

    /// Check if this column is a primary key
    fn is_primary_key(&self) -> bool {
        false
    }

    /// Check if this column is auto-increment
    fn is_auto_increment(&self) -> bool {
        false
    }

    /// Get the default value SQL expression (if any)
    fn default_value(&self) -> Option<&'static str> {
        None
    }

    /// Check if this column is unique
    fn is_unique(&self) -> bool {
        false
    }

    /// Get the old column name if this column was renamed (for migrations)
    ///
    /// When a column is renamed, this returns the previous name so that
    /// migrations can perform a `RENAME COLUMN` instead of dropping and
    /// recreating the column, preserving the data.
    fn renamed_from(&self) -> Option<&'static str> {
        None
    }

    /// Get the foreign key information if this column is a foreign key
    fn foreign_key(&self) -> Option<ForeignKeyInfo> {
        None
    }

    /// Get all columns as a static slice
    fn all() -> &'static [Self];
}
