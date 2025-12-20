use std::marker::PhantomData;

use crate::ChangeSetTrait;
use crate::Error;
use crate::Result;
use crate::TableTrait;
use crate::Value;

#[derive(Clone, Debug)]
pub struct Insert<Table: TableTrait> {
    change_sets: Vec<Table::ChangeSet>,
    _table:      PhantomData<Table>,
}

impl<Table: TableTrait> Insert<Table> {
    pub fn new(change_set: Table::ChangeSet) -> Self {
        Self { change_sets: vec![change_set], _table: PhantomData }
    }

    pub fn empty() -> Self {
        Self { change_sets: Vec::new(), _table: PhantomData }
    }

    pub fn add(mut self, change_set: Table::ChangeSet) -> Self {
        self.change_sets.push(change_set);
        self
    }

    pub fn add_many(mut self, change_sets: impl IntoIterator<Item = Table::ChangeSet>) -> Self {
        self.change_sets.extend(change_sets);
        self
    }

    fn build_single(&self, change_set: &Table::ChangeSet) -> (String, Vec<Value>) {
        let (columns, values) = change_set.get_insert_columns_and_values();

        if columns.is_empty() {
            return (format!("INSERT INTO {} DEFAULT VALUES", Table::table_name()), Vec::new());
        }

        let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            Table::table_name(),
            columns.join(", "),
            placeholders.join(", ")
        );

        (sql, values)
    }

    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
        if self.change_sets.is_empty() {
            return Ok(0);
        }

        let mut total_affected = 0u64;

        for change_set in &self.change_sets {
            let (sql, params) = self.build_single(change_set);
            let params: Vec<turso::Value> = params.into_iter().collect();
            let affected = conn.execute(&sql, params).await?;
            total_affected += affected;
        }

        Ok(total_affected)
    }

    pub async fn exec_with_last_insert_id(self, conn: &crate::Connection) -> Result<i64> {
        if self.change_sets.is_empty() {
            return Err(Error::Query("No recrods to insert".to_string()));
        }

        let change_set = self.change_sets.first().unwrap();
        let (sql, params) = self.build_single(change_set);
        tracing::debug!("Insert SQL: {}", sql);
        tracing::debug!("Insert Params: {:?}", params);

        conn.execute(&sql, params).await?;
        Ok(conn.last_insert_rowid())
    }
}

#[derive(Clone, Debug)]
pub struct InsertMany<Table: TableTrait> {
    change_sets: Vec<Table::ChangeSet>,
    _table:      PhantomData<Table>,
}

impl<Table: TableTrait> InsertMany<Table> {
    pub fn new(change_sets: Vec<Table::ChangeSet>) -> Self {
        Self { change_sets, _table: PhantomData }
    }

    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
        if self.change_sets.is_empty() {
            return Ok(0);
        }

        let mut total_affected = 0u64;

        for change_set in &self.change_sets {
            let (columns, values) = change_set.get_insert_columns_and_values();

            let sql = if columns.is_empty() {
                format!("INSERT INTO {} DEFAULT VALUES", Table::table_name())
            } else {
                let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
                format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    Table::table_name(),
                    columns.join(", "),
                    placeholders.join(", ")
                )
            };

            let params: Vec<turso::Value> = values.into_iter().collect();
            let affected = conn.execute(&sql, params).await?;
            total_affected += affected;
        }

        Ok(total_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColumnTrait;
    use crate::ColumnType;
    use crate::FieldValue;
    use crate::FromRow;
    #[allow(unused_imports)]
    use crate::IntoValue;
    use crate::RecordTrait;
    use crate::set;

    #[derive(Clone, Debug, PartialEq)]
    struct TestRecord {
        id:    i64,
        name:  String,
        email: String,
    }

    impl RecordTrait for TestRecord {
        type Table = TestTable;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for TestRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(TestRecord { id: 1, name: "test".to_string(), email: "test@test.com".to_string() })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestChangeSet {
        id:    FieldValue<i64>,
        name:  FieldValue<String>,
        email: FieldValue<String>,
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
                values.push(Value::Text(self.email.clone().take().unwrap()));
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            let mut sets = Vec::new();
            if self.name.is_changed() {
                sets.push(("name", Value::Text(self.name.clone().take().unwrap())));
            }
            if self.email.is_changed() {
                sets.push(("email", Value::Text(self.email.clone().take().unwrap())));
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
            }
        }

        fn column_type(&self) -> ColumnType {
            match self {
                TestColumn::Id => ColumnType::Integer,
                TestColumn::Name | TestColumn::Email => ColumnType::Text,
            }
        }

        fn all() -> &'static [Self] {
            &[TestColumn::Id, TestColumn::Name, TestColumn::Email]
        }
    }

    #[derive(Default, Clone, Debug)]
    struct TestTable;

    impl TableTrait for TestTable {
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
            "id, name, email"
        }

        fn column_count() -> usize {
            3
        }
    }

    #[test]
    fn test_insert_new() {
        let change_set = TestChangeSet {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestTable>::new(change_set);

        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_empty() {
        let insert = Insert::<TestTable>::empty();

        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_add() {
        let change_set1 = TestChangeSet {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let change_set2 = TestChangeSet {
            name: set("Bob".to_string()),
            email: set("bob@example.com".to_string()),
            ..Default::default()
        };

        let insert = Insert::<TestTable>::new(change_set1).add(change_set2);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_add_many() {
        let change_sets = vec![
            TestChangeSet {
                name: set("Alice".to_string()),
                email: set("alice@example.com".to_string()),
                ..Default::default()
            },
            TestChangeSet {
                name: set("Bob".to_string()),
                email: set("bob@example.com".to_string()),
                ..Default::default()
            },
        ];

        let insert = Insert::<TestTable>::empty().add_many(change_sets);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_build_with_values() {
        let change_set = TestChangeSet {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestTable>::new(change_set);

        assert!(format!("{:?}", insert).contains("Alice"));
    }

    #[test]
    fn test_insert_with_empty_change_set() {
        let change_set = TestChangeSet::default();
        let insert = Insert::<TestTable>::new(change_set);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_clone() {
        let change_set = TestChangeSet {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestTable>::new(change_set);
        let cloned = insert.clone();

        assert_eq!(format!("{:?}", insert), format!("{:?}", cloned));
    }

    #[test]
    fn test_insert_debug() {
        let change_set = TestChangeSet { name: set("Test".to_string()), ..Default::default() };
        let insert = Insert::<TestTable>::new(change_set);
        let debug = format!("{:?}", insert);

        assert!(debug.contains("Insert"));
    }

    #[test]
    fn test_insert_many_new() {
        let change_sets = vec![
            TestChangeSet {
                name: set("Alice".to_string()),
                email: set("alice@example.com".to_string()),
                ..Default::default()
            },
            TestChangeSet {
                name: set("Bob".to_string()),
                email: set("bob@example.com".to_string()),
                ..Default::default()
            },
        ];

        let insert_many = InsertMany::<TestTable>::new(change_sets);
        assert!(format!("{:?}", insert_many).contains("InsertMany"));
    }

    #[test]
    fn test_insert_many_empty() {
        let insert_many = InsertMany::<TestTable>::new(vec![]);
        assert!(format!("{:?}", insert_many).contains("InsertMany"));
    }

    #[test]
    fn test_insert_many_clone() {
        let change_sets = vec![TestChangeSet { name: set("Alice".to_string()), ..Default::default() }];
        let insert_many = InsertMany::<TestTable>::new(change_sets);
        let cloned = insert_many.clone();

        assert_eq!(format!("{:?}", insert_many), format!("{:?}", cloned));
    }

    #[test]
    fn test_insert_partial_fields() {
        let change_set = TestChangeSet { name: set("Alice".to_string()), ..Default::default() };
        let insert = Insert::<TestTable>::new(change_set);
        let debug = format!("{:?}", insert);
        assert!(debug.contains("Alice"));

        assert!(debug.contains("NotSet"));
    }

    #[test]
    fn test_insert_chained_add() {
        let insert = Insert::<TestTable>::empty()
            .add(TestChangeSet { name: set("Alice".to_string()), ..Default::default() })
            .add(TestChangeSet { name: set("Bob".to_string()), ..Default::default() })
            .add(TestChangeSet { name: set("Charlie".to_string()), ..Default::default() });

        let debug = format!("{:?}", insert);
        assert!(debug.contains("Alice"));
        assert!(debug.contains("Bob"));
        assert!(debug.contains("Charlie"));
    }
}
