use crate::value::ColumnType;

// Not yet implemented, ignored
#[derive(Debug, Clone, Copy, Default)]
pub enum OnDelete {
    Restrict,
    #[default]
    Cascade,
    SetNull,
    SetDefault,
    None,
}

// Not yet implemented, ignored
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

pub trait ColumnTrait: std::fmt::Debug + Copy + Clone + std::fmt::Display + 'static {
    fn name(&self) -> &'static str;

    fn column_type(&self) -> ColumnType;

    fn is_nullable(&self) -> bool {
        false
    }

    fn is_primary_key(&self) -> bool {
        false
    }

    fn is_auto_increment(&self) -> bool {
        false
    }

    fn default_value(&self) -> Option<&'static str> {
        None
    }

    fn is_unique(&self) -> bool {
        false
    }

    fn renamed_from(&self) -> Option<&'static str> {
        None
    }

    fn foreign_key(&self) -> Option<ForeignKeyInfo> {
        None
    }

    fn all() -> &'static [Self];
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RowIdColumn;

impl ColumnTrait for RowIdColumn {
    fn name(&self) -> &'static str {
        "rowid"
    }

    fn column_type(&self) -> ColumnType {
        ColumnType::Integer
    }

    fn is_primary_key(&self) -> bool {
        true
    }

    fn is_auto_increment(&self) -> bool {
        true
    }

    fn default_value(&self) -> Option<&'static str> {
        Some("1")
    }

    fn is_unique(&self) -> bool {
        true
    }

    fn renamed_from(&self) -> Option<&'static str> {
        None
    }

    fn foreign_key(&self) -> Option<ForeignKeyInfo> {
        None
    }

    fn all() -> &'static [Self] {
        &[RowIdColumn]
    }
}

impl std::fmt::Display for RowIdColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rowid")
    }
}
