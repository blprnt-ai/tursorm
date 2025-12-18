//! Database connection wrapper for tursorm

use crate::ColumnTrait;
use crate::EntityTrait;
use crate::Result;

/// Schema helper for creating and dropping tables
///
/// Provides static methods to generate DDL (Data Definition Language)
/// SQL statements for entity tables.
///
/// # Example
///
/// ```ignore
/// // Generate CREATE TABLE SQL
/// let sql = Schema::create_table_sql::<UserEntity>(false);
/// conn.execute(&sql, ()).await?;
///
/// // Or use the async helper
/// Schema::create_table::<UserEntity>(&conn, true).await?;
/// ```
pub struct Schema;

impl Schema {
    /// Create a table for an entity
    ///
    /// This generates a CREATE TABLE statement based on the entity's column metadata.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Schema::create_table::<UserEntity>(&conn, true).await?;
    /// ```
    pub async fn create_table<E: EntityTrait>(conn: &crate::Connection, if_not_exists: bool) -> Result<()>
    where E::Column: 'static {
        let sql = Self::create_table_sql::<E>(if_not_exists);
        conn.execute(&sql, ()).await?;
        Ok(())
    }

    /// Generate the CREATE TABLE SQL statement for an entity
    pub fn create_table_sql<E: EntityTrait>(if_not_exists: bool) -> String
    where E::Column: 'static {
        let exists_clause = if if_not_exists { "IF NOT EXISTS " } else { "" };

        let mut column_defs = Vec::new();
        let mut primary_keys = Vec::new();

        for col in <E::Column as ColumnTrait>::all() {
            let mut def = format!("{} {}", col.name(), column_type_to_sql(col.column_type()));

            // Handle PRIMARY KEY
            if col.is_primary_key() {
                primary_keys.push(col.name().to_string());

                // For single primary key with auto_increment, add inline
                if col.is_auto_increment() {
                    def.push_str(" PRIMARY KEY AUTOINCREMENT");
                }
            }

            // Handle NOT NULL (non-nullable columns)
            if !col.is_nullable() && !col.is_primary_key() {
                def.push_str(" NOT NULL");
            }

            // Handle UNIQUE
            if col.is_unique() && !col.is_primary_key() {
                def.push_str(" UNIQUE");
            }

            // Handle DEFAULT
            if let Some(default) = col.default_value() {
                def.push_str(&format!(" DEFAULT {}", default));
            }

            column_defs.push(def);
        }

        // Add composite primary key constraint if multiple primary keys or non-autoincrement single PK
        let needs_pk_constraint = primary_keys.len() > 1
            || (primary_keys.len() == 1
                && !E::Column::all()
                    .iter()
                    .find(|c| c.is_primary_key())
                    .map(|c| c.is_auto_increment())
                    .unwrap_or(false));

        if needs_pk_constraint && !primary_keys.is_empty() {
            // Only add if we didn't already add inline PRIMARY KEY
            let has_inline_pk = E::Column::all().iter().any(|c| c.is_primary_key() && c.is_auto_increment());

            if !has_inline_pk {
                column_defs.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
            }
        }

        format!("CREATE TABLE {}{} (\n  {}\n)", exists_clause, E::table_name(), column_defs.join(",\n  "))
    }

    /// Drop a table
    pub async fn drop_table<E: EntityTrait>(conn: &crate::Connection, if_exists: bool) -> Result<()> {
        let sql = Self::drop_table_sql::<E>(if_exists);
        conn.execute(&sql, ()).await?;
        Ok(())
    }

    /// Generate the DROP TABLE SQL statement
    pub fn drop_table_sql<E: EntityTrait>(if_exists: bool) -> String {
        let exists_clause = if if_exists { "IF EXISTS " } else { "" };
        format!("DROP TABLE {}{}", exists_clause, E::table_name())
    }

    /// Check if a table exists
    pub async fn table_exists<E: EntityTrait>(conn: &crate::Connection) -> Result<bool> {
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

/// Convert a ColumnType to its SQL representation
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
    use crate::ActiveModelTrait;
    use crate::ActiveValue;
    use crate::FromRow;
    use crate::Value;
    use crate::value::ColumnType;

    // Test helpers
    fn column_type_to_sql_test(col_type: ColumnType) -> &'static str {
        column_type_to_sql(col_type)
    }

    // column_type_to_sql tests
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

    // Mock Entity for Schema tests
    #[derive(Clone, Debug, PartialEq)]
    struct TestModel {
        id:    i64,
        name:  String,
        email: Option<String>,
        age:   i64,
    }

    impl crate::ModelTrait for TestModel {
        type Entity = TestEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for TestModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(TestModel { id: 1, name: "test".to_string(), email: Some("test@test.com".to_string()), age: 25 })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestActiveModel {
        id:    ActiveValue<i64>,
        name:  ActiveValue<String>,
        email: ActiveValue<Option<String>>,
        age:   ActiveValue<i64>,
    }

    impl ActiveModelTrait for TestActiveModel {
        type Entity = TestEntity;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            let mut columns = Vec::new();
            let mut values = Vec::new();
            if self.name.is_set() {
                columns.push("name");
                values.push(Value::Text(self.name.clone().take().unwrap()));
            }
            if self.email.is_set() {
                columns.push("email");
                if let Some(email) = self.email.clone().take().unwrap() {
                    values.push(Value::Text(email));
                } else {
                    values.push(Value::Null);
                }
            }
            if self.age.is_set() {
                columns.push("age");
                values.push(Value::Integer(self.age.clone().take().unwrap()));
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            let mut sets = Vec::new();
            if self.name.is_set() {
                sets.push(("name", Value::Text(self.name.clone().take().unwrap())));
            }
            if self.email.is_set() {
                if let Some(email) = self.email.clone().take().unwrap() {
                    sets.push(("email", Value::Text(email)));
                } else {
                    sets.push(("email", Value::Null));
                }
            }
            if self.age.is_set() {
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
    struct TestEntity;

    impl crate::EntityTrait for TestEntity {
        type ActiveModel = TestActiveModel;
        type Column = TestColumn;
        type Model = TestModel;

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

    // Schema::create_table_sql tests
    #[test]
    fn test_schema_create_table_sql_basic() {
        let sql = Schema::create_table_sql::<TestEntity>(false);

        assert!(sql.contains("CREATE TABLE test_users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("name TEXT NOT NULL"));
        assert!(sql.contains("email TEXT"));
        assert!(sql.contains("age INTEGER NOT NULL"));
    }

    #[test]
    fn test_schema_create_table_sql_if_not_exists() {
        let sql = Schema::create_table_sql::<TestEntity>(true);

        assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_users"));
    }

    #[test]
    fn test_schema_create_table_sql_no_if_not_exists() {
        let sql = Schema::create_table_sql::<TestEntity>(false);

        assert!(!sql.contains("IF NOT EXISTS"));
    }

    // Schema::drop_table_sql tests
    #[test]
    fn test_schema_drop_table_sql_basic() {
        let sql = Schema::drop_table_sql::<TestEntity>(false);

        assert_eq!(sql, "DROP TABLE test_users");
    }

    #[test]
    fn test_schema_drop_table_sql_if_exists() {
        let sql = Schema::drop_table_sql::<TestEntity>(true);

        assert_eq!(sql, "DROP TABLE IF EXISTS test_users");
    }

    // Test Schema struct creation
    #[test]
    fn test_schema_struct() {
        // Schema is a unit struct, so we just verify it exists
        let _schema = Schema;
    }

    // Entity with unique column test
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
    struct UniqueTestModel {
        id:    i64,
        email: String,
    }

    impl crate::ModelTrait for UniqueTestModel {
        type Entity = UniqueTestEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for UniqueTestModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(UniqueTestModel { id: 1, email: "test@test.com".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct UniqueTestActiveModel {
        id: ActiveValue<i64>,
    }

    impl ActiveModelTrait for UniqueTestActiveModel {
        type Entity = UniqueTestEntity;

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
    struct UniqueTestEntity;

    impl crate::EntityTrait for UniqueTestEntity {
        type ActiveModel = UniqueTestActiveModel;
        type Column = UniqueTestColumn;
        type Model = UniqueTestModel;

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
        let sql = Schema::create_table_sql::<UniqueTestEntity>(false);

        // email should be NOT NULL UNIQUE since it's not nullable
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    }

    // Entity with default value test
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
    struct DefaultTestModel {
        id:     i64,
        status: String,
    }

    impl crate::ModelTrait for DefaultTestModel {
        type Entity = DefaultTestEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for DefaultTestModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(DefaultTestModel { id: 1, status: "active".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct DefaultTestActiveModel {
        id: ActiveValue<i64>,
    }

    impl ActiveModelTrait for DefaultTestActiveModel {
        type Entity = DefaultTestEntity;

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
    struct DefaultTestEntity;

    impl crate::EntityTrait for DefaultTestEntity {
        type ActiveModel = DefaultTestActiveModel;
        type Column = DefaultTestColumn;
        type Model = DefaultTestModel;

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
        let sql = Schema::create_table_sql::<DefaultTestEntity>(false);

        assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
    }

    // Test composite primary key (non-autoincrement)
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
    struct CompositeModel {
        user_id: i64,
        post_id: i64,
        content: String,
    }

    impl crate::ModelTrait for CompositeModel {
        type Entity = CompositeEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.user_id)
        }
    }

    impl FromRow for CompositeModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(CompositeModel { user_id: 1, post_id: 1, content: "test".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct CompositeActiveModel {
        user_id: ActiveValue<i64>,
    }

    impl ActiveModelTrait for CompositeActiveModel {
        type Entity = CompositeEntity;

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
    struct CompositeEntity;

    impl crate::EntityTrait for CompositeEntity {
        type ActiveModel = CompositeActiveModel;
        type Column = CompositeColumn;
        type Model = CompositeModel;

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
        let sql = Schema::create_table_sql::<CompositeEntity>(false);

        // Should have PRIMARY KEY constraint at the end, not inline AUTOINCREMENT
        assert!(sql.contains("PRIMARY KEY (user_id)"));
        assert!(!sql.contains("AUTOINCREMENT"));
    }
}
