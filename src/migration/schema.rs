use crate::ColumnTrait;
use crate::Result;
use crate::TableTrait;

pub struct MigrationSchema;

impl MigrationSchema {
    pub async fn create_table<E: TableTrait>(conn: &crate::Connection, if_not_exists: bool) -> Result<()>
    where E::Column: 'static {
        let sql = Self::create_table_sql::<E>(if_not_exists);
        conn.execute(&sql, ()).await?;
        Ok(())
    }

    pub fn create_table_sql<E: TableTrait>(if_not_exists: bool) -> String
    where E::Column: 'static {
        let exists_clause = if if_not_exists { "IF NOT EXISTS " } else { "" };

        let mut column_defs = Vec::new();
        let mut primary_keys = Vec::new();

        for col in <E::Column as ColumnTrait>::all() {
            let mut def = format!("{} {}", col.name(), column_type_to_sql(col.column_type()));

            if col.is_primary_key() {
                primary_keys.push(col.name().to_string());

                if col.is_auto_increment() {
                    def.push_str(" PRIMARY KEY AUTOINCREMENT");
                }
            }

            if !col.is_nullable() && !col.is_primary_key() {
                def.push_str(" NOT NULL");
            }

            if col.is_unique() && !col.is_primary_key() {
                def.push_str(" UNIQUE");
            }

            if let Some(default) = col.default_value() {
                def.push_str(&format!(" DEFAULT {}", default));
            }

            column_defs.push(def);
        }

        let needs_pk_constraint = primary_keys.len() > 1
            || (primary_keys.len() == 1
                && !E::Column::all()
                    .iter()
                    .find(|c| c.is_primary_key())
                    .map(|c| c.is_auto_increment())
                    .unwrap_or(false));

        if needs_pk_constraint && !primary_keys.is_empty() {
            let has_inline_pk = E::Column::all().iter().any(|c| c.is_primary_key() && c.is_auto_increment());

            if !has_inline_pk {
                column_defs.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
            }
        }

        format!("CREATE TABLE {}{} (\n  {}\n)", exists_clause, E::table_name(), column_defs.join(",\n  "))
    }

    pub async fn drop_table<E: TableTrait>(conn: &crate::Connection, if_exists: bool) -> Result<()> {
        let sql = Self::drop_table_sql::<E>(if_exists);
        conn.execute(&sql, ()).await?;
        Ok(())
    }

    pub fn drop_table_sql<E: TableTrait>(if_exists: bool) -> String {
        let exists_clause = if if_exists { "IF EXISTS " } else { "" };
        format!("DROP TABLE {}{}", exists_clause, E::table_name())
    }

    pub async fn table_exists<E: TableTrait>(conn: &crate::Connection) -> Result<bool> {
        let sql = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?";

        let mut rows = conn.query(sql, [E::table_name()]).await?;

        if let Some(row) = rows.next().await? {
            let value = row.get_value(0)?;
            match value {
                crate::Value::Integer(count) => Ok(count > 0),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

fn column_type_to_sql(col_type: crate::value::ColumnType) -> &'static str {
    match col_type {
        crate::value::ColumnType::Integer => "INTEGER",
        crate::value::ColumnType::Float => "REAL",
        crate::value::ColumnType::Text => "TEXT",
        crate::value::ColumnType::Blob => "BLOB",
        crate::value::ColumnType::Null => "NULL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeSetTrait;
    use crate::FieldValue;
    use crate::FromRow;
    use crate::Value;
    use crate::value::ColumnType;

    fn column_type_to_sql_test(col_type: ColumnType) -> &'static str {
        column_type_to_sql(col_type)
    }

    #[test]
    fn test_column_type_to_sql_integer() {
        assert_eq!(column_type_to_sql_test(ColumnType::Integer), "INTEGER");
    }

    #[test]
    fn test_column_type_to_sql_float() {
        assert_eq!(column_type_to_sql_test(ColumnType::Float), "REAL");
    }

    #[test]
    fn test_column_type_to_sql_text() {
        assert_eq!(column_type_to_sql_test(ColumnType::Text), "TEXT");
    }

    #[test]
    fn test_column_type_to_sql_blob() {
        assert_eq!(column_type_to_sql_test(ColumnType::Blob), "BLOB");
    }

    #[test]
    fn test_column_type_to_sql_null() {
        assert_eq!(column_type_to_sql_test(ColumnType::Null), "NULL");
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TestRecord {
        id:    i64,
        name:  String,
        email: Option<String>,
        age:   i64,
    }

    impl crate::RecordTrait for TestRecord {
        type Table = TestTable;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for TestRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(TestRecord { id: 1, name: "test".to_string(), email: Some("test@test.com".to_string()), age: 25 })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestChangeSet {
        id:    FieldValue<i64>,
        name:  FieldValue<String>,
        email: FieldValue<Option<String>>,
        age:   FieldValue<i64>,
    }

    impl ChangeSetTrait for TestChangeSet {
        type Table = TestTable;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            let mut columns = Vec::new();
            let mut values = Vec::new();
            if self.name.is_changed() {
                columns.push("name");
                values.push(Value::Text(self.name.clone().take().unwrap()));
            }
            if self.email.is_changed() {
                columns.push("email");
                if let Some(email) = self.email.clone().take().unwrap() {
                    values.push(Value::Text(email));
                } else {
                    values.push(Value::Null);
                }
            }
            if self.age.is_changed() {
                columns.push("age");
                values.push(Value::Integer(self.age.clone().take().unwrap()));
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            let mut sets = Vec::new();
            if self.name.is_changed() {
                sets.push(("name", Value::Text(self.name.clone().take().unwrap())));
            }
            if self.email.is_changed() {
                if let Some(email) = self.email.clone().take().unwrap() {
                    sets.push(("email", Value::Text(email)));
                } else {
                    sets.push(("email", Value::Null));
                }
            }
            if self.age.is_changed() {
                sets.push(("age", Value::Integer(self.age.clone().take().unwrap())));
            }
            sets
        }

        fn get_primary_key_value(&self) -> Option<Value> {
            self.id.clone().take().map(|v| Value::Integer(v))
        }

        fn primary_key_column() -> &'static str {
            "id"
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum TestColumn {
        Id,
        Name,
        Email,
        Age,
    }

    impl std::fmt::Display for TestColumn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    impl ColumnTrait for TestColumn {
        fn name(&self) -> &'static str {
            match self {
                TestColumn::Id => "id",
                TestColumn::Name => "name",
                TestColumn::Email => "email",
                TestColumn::Age => "age",
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                TestColumn::Id | TestColumn::Age => ColumnType::Integer,
                TestColumn::Name | TestColumn::Email => ColumnType::Text,
            }
        }

        fn is_nullable(&self) -> bool {
            matches!(self, TestColumn::Email)
        }

        fn is_primary_key(&self) -> bool {
            matches!(self, TestColumn::Id)
        }

        fn is_auto_increment(&self) -> bool {
            matches!(self, TestColumn::Id)
        }

        fn all() -> &'static [Self] {
            &[TestColumn::Id, TestColumn::Name, TestColumn::Email, TestColumn::Age]
        }
    }

    #[derive(Default)]
    struct TestTable;

    impl crate::TableTrait for TestTable {
        type ChangeSet = TestChangeSet;
        type Column = TestColumn;
        type Record = TestRecord;

        fn table_name() -> &'static str {
            "test_users"
        }

        fn primary_key() -> Self::Column {
            TestColumn::Id
        }

        fn primary_key_auto_increment() -> bool {
            true
        }

        fn all_columns() -> &'static str {
            "id, name, email, age"
        }

        fn column_count() -> usize {
            4
        }
    }

    #[test]
    fn test_schema_create_table_sql_basic() {
        let sql = MigrationSchema::create_table_sql::<TestTable>(false);

        assert!(sql.contains("CREATE TABLE test_users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("name TEXT NOT NULL"));
        assert!(sql.contains("email TEXT"));
        assert!(sql.contains("age INTEGER NOT NULL"));
    }

    #[test]
    fn test_schema_create_table_sql_if_not_exists() {
        let sql = MigrationSchema::create_table_sql::<TestTable>(true);

        assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_users"));
    }

    #[test]
    fn test_schema_create_table_sql_no_if_not_exists() {
        let sql = MigrationSchema::create_table_sql::<TestTable>(false);

        assert!(!sql.contains("IF NOT EXISTS"));
    }

    #[test]
    fn test_schema_drop_table_sql_basic() {
        let sql = MigrationSchema::drop_table_sql::<TestTable>(false);

        assert_eq!(sql, "DROP TABLE test_users");
    }

    #[test]
    fn test_schema_drop_table_sql_if_exists() {
        let sql = MigrationSchema::drop_table_sql::<TestTable>(true);

        assert_eq!(sql, "DROP TABLE IF EXISTS test_users");
    }

    #[test]
    fn test_schema_struct() {
        let _schema = MigrationSchema;
    }

    #[derive(Clone, Copy, Debug)]
    enum UniqueTestColumn {
        Id,
        Email,
    }

    impl std::fmt::Display for UniqueTestColumn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    impl ColumnTrait for UniqueTestColumn {
        fn name(&self) -> &'static str {
            match self {
                UniqueTestColumn::Id => "id",
                UniqueTestColumn::Email => "email",
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                UniqueTestColumn::Id => ColumnType::Integer,
                UniqueTestColumn::Email => ColumnType::Text,
            }
        }

        fn is_primary_key(&self) -> bool {
            matches!(self, UniqueTestColumn::Id)
        }

        fn is_auto_increment(&self) -> bool {
            matches!(self, UniqueTestColumn::Id)
        }

        fn is_unique(&self) -> bool {
            matches!(self, UniqueTestColumn::Email)
        }

        fn all() -> &'static [Self] {
            &[UniqueTestColumn::Id, UniqueTestColumn::Email]
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct UniqueTestRecord {
        id:    i64,
        email: String,
    }

    impl crate::RecordTrait for UniqueTestRecord {
        type Table = UniqueTestTable;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for UniqueTestRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(UniqueTestRecord { id: 1, email: "test@test.com".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct UniqueTestChangeSet {
        id: FieldValue<i64>,
    }

    impl ChangeSetTrait for UniqueTestChangeSet {
        type Table = UniqueTestTable;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            (vec![], vec![])
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            vec![]
        }

        fn get_primary_key_value(&self) -> Option<Value> {
            self.id.clone().take().map(|v| Value::Integer(v))
        }

        fn primary_key_column() -> &'static str {
            "id"
        }
    }

    #[derive(Default)]
    struct UniqueTestTable;

    impl crate::TableTrait for UniqueTestTable {
        type ChangeSet = UniqueTestChangeSet;
        type Column = UniqueTestColumn;
        type Record = UniqueTestRecord;

        fn table_name() -> &'static str {
            "unique_test"
        }

        fn primary_key() -> Self::Column {
            UniqueTestColumn::Id
        }

        fn primary_key_auto_increment() -> bool {
            true
        }

        fn all_columns() -> &'static str {
            "id, email"
        }

        fn column_count() -> usize {
            2
        }
    }

    #[test]
    fn test_schema_create_table_with_unique() {
        let sql = MigrationSchema::create_table_sql::<UniqueTestTable>(false);

        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    }

    #[derive(Clone, Copy, Debug)]
    enum DefaultTestColumn {
        Id,
        Status,
    }

    impl std::fmt::Display for DefaultTestColumn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    impl ColumnTrait for DefaultTestColumn {
        fn name(&self) -> &'static str {
            match self {
                DefaultTestColumn::Id => "id",
                DefaultTestColumn::Status => "status",
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                DefaultTestColumn::Id => ColumnType::Integer,
                DefaultTestColumn::Status => ColumnType::Text,
            }
        }

        fn is_primary_key(&self) -> bool {
            matches!(self, DefaultTestColumn::Id)
        }

        fn is_auto_increment(&self) -> bool {
            matches!(self, DefaultTestColumn::Id)
        }

        fn default_value(&self) -> Option<&'static str> {
            match self {
                DefaultTestColumn::Status => Some("'active'"),
                _ => None,
            }
        }

        fn all() -> &'static [Self] {
            &[DefaultTestColumn::Id, DefaultTestColumn::Status]
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct DefaultTestRecord {
        id:     i64,
        status: String,
    }

    impl crate::RecordTrait for DefaultTestRecord {
        type Table = DefaultTestTable;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for DefaultTestRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(DefaultTestRecord { id: 1, status: "active".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct DefaultTestChangeSet {
        id: FieldValue<i64>,
    }

    impl ChangeSetTrait for DefaultTestChangeSet {
        type Table = DefaultTestTable;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            (vec![], vec![])
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            vec![]
        }

        fn get_primary_key_value(&self) -> Option<Value> {
            self.id.clone().take().map(|v| Value::Integer(v))
        }

        fn primary_key_column() -> &'static str {
            "id"
        }
    }

    #[derive(Default)]
    struct DefaultTestTable;

    impl crate::TableTrait for DefaultTestTable {
        type ChangeSet = DefaultTestChangeSet;
        type Column = DefaultTestColumn;
        type Record = DefaultTestRecord;

        fn table_name() -> &'static str {
            "default_test"
        }

        fn primary_key() -> Self::Column {
            DefaultTestColumn::Id
        }

        fn primary_key_auto_increment() -> bool {
            true
        }

        fn all_columns() -> &'static str {
            "id, status"
        }

        fn column_count() -> usize {
            2
        }
    }

    #[test]
    fn test_schema_create_table_with_default() {
        let sql = MigrationSchema::create_table_sql::<DefaultTestTable>(false);

        assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
    }

    #[derive(Clone, Copy, Debug)]
    enum CompositeColumn {
        UserId,
        PostId,
        Content,
    }

    impl std::fmt::Display for CompositeColumn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    impl ColumnTrait for CompositeColumn {
        fn name(&self) -> &'static str {
            match self {
                CompositeColumn::UserId => "user_id",
                CompositeColumn::PostId => "post_id",
                CompositeColumn::Content => "content",
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                CompositeColumn::UserId | CompositeColumn::PostId => ColumnType::Integer,
                CompositeColumn::Content => ColumnType::Text,
            }
        }

        fn is_primary_key(&self) -> bool {
            matches!(self, CompositeColumn::UserId)
        }

        fn is_auto_increment(&self) -> bool {
            false
        }

        fn all() -> &'static [Self] {
            &[CompositeColumn::UserId, CompositeColumn::PostId, CompositeColumn::Content]
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct CompositeRecord {
        user_id: i64,
        post_id: i64,
        content: String,
    }

    impl crate::RecordTrait for CompositeRecord {
        type Table = CompositeTable;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.user_id)
        }
    }

    impl FromRow for CompositeRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(CompositeRecord { user_id: 1, post_id: 1, content: "test".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct CompositeChangeSet {
        user_id: FieldValue<i64>,
    }

    impl ChangeSetTrait for CompositeChangeSet {
        type Table = CompositeTable;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            (vec![], vec![])
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            vec![]
        }

        fn get_primary_key_value(&self) -> Option<Value> {
            self.user_id.clone().take().map(|v| Value::Integer(v))
        }

        fn primary_key_column() -> &'static str {
            "user_id"
        }
    }

    #[derive(Default)]
    struct CompositeTable;

    impl crate::TableTrait for CompositeTable {
        type ChangeSet = CompositeChangeSet;
        type Column = CompositeColumn;
        type Record = CompositeRecord;

        fn table_name() -> &'static str {
            "composite_test"
        }

        fn primary_key() -> Self::Column {
            CompositeColumn::UserId
        }

        fn primary_key_auto_increment() -> bool {
            false
        }

        fn all_columns() -> &'static str {
            "user_id, post_id, content"
        }

        fn column_count() -> usize {
            3
        }
    }

    #[test]
    fn test_schema_create_table_non_autoincrement_pk() {
        let sql = MigrationSchema::create_table_sql::<CompositeTable>(false);

        assert!(sql.contains("PRIMARY KEY (user_id)"));
        assert!(!sql.contains("AUTOINCREMENT"));
    }
}
