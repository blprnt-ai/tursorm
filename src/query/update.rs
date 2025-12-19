use std::marker::PhantomData;

use crate::ChangeSetTrait;
use crate::ColumnTrait;
use crate::Condition;
use crate::Error;
use crate::FromRow;
use crate::IntoValue;
use crate::Result;
use crate::TableTrait;
use crate::Value;

#[derive(Clone, Debug)]
pub struct Update<Table: TableTrait> {
    change_set: Option<Table::ChangeSet>,
    changes:    Vec<(String, Value)>,
    conditions: Vec<Condition>,
    _table:     PhantomData<Table>,
}

impl<Table: TableTrait> Update<Table> {
    pub fn new(change_set: Table::ChangeSet) -> Self {
        Self { change_set: Some(change_set), changes: Vec::new(), conditions: Vec::new(), _table: PhantomData }
    }

    pub fn many() -> Self {
        Self { change_set: None, changes: Vec::new(), conditions: Vec::new(), _table: PhantomData }
    }

    pub fn set<Column: ColumnTrait, Value: IntoValue>(mut self, column: Column, value: Value) -> Self {
        self.changes.push((column.name().to_string(), value.into_value()));
        self
    }

    pub fn filter(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    fn build(&self) -> Result<(String, Vec<Value>)> {
        let mut set_parts = Vec::new();
        let mut params = Vec::new();

        if let Some(ref change_set) = self.change_set {
            let change_set_changes = change_set.get_update_sets();
            for (col, val) in change_set_changes {
                set_parts.push(format!("{} = ?", col));
                params.push(val);
            }
        }

        for (col, val) in &self.changes {
            set_parts.push(format!("{} = ?", col));
            params.push(val.clone());
        }

        if set_parts.is_empty() {
            return Err(Error::Query("No columns to update".to_string()));
        }

        let mut sql = format!("UPDATE {} SET {}", Table::table_name(), set_parts.join(", "));

        let mut where_conditions = self.conditions.clone();

        if let Some(ref change_set) = self.change_set {
            if let Some(pk_value) = change_set.get_primary_key_value() {
                let pk_column = Table::ChangeSet::primary_key_column();
                where_conditions.push(Condition::raw(format!("{} = ?", pk_column), vec![pk_value]));
            } else if self.conditions.is_empty() {
                return Err(Error::PrimaryKeyNotSet);
            }
        }

        if !where_conditions.is_empty() {
            let where_parts: Vec<String> = where_conditions.iter().map(|c| format!("({})", c.sql())).collect();
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));

            for condition in &where_conditions {
                params.extend(condition.values().iter().cloned());
            }
        }

        Ok((sql, params))
    }

    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
        let (sql, params) = self.build()?;
        let params: Vec<turso::Value> = params.into_iter().collect();
        let affected = conn.execute(&sql, params).await?;
        Ok(affected)
    }

    pub async fn exec_with_returning(self, conn: &crate::Connection) -> Result<Table::Record> {
        let (base_sql, params) = self.build()?;
        let sql = format!("{} RETURNING {}", base_sql, Table::all_columns());

        let params: Vec<turso::Value> = params.into_iter().collect();
        let mut rows = conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? { Table::Record::from_row(&row) } else { Err(Error::NoRowsAffected) }
    }
}

impl<Table: TableTrait> Default for Update<Table> {
    fn default() -> Self {
        Self::many()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColumnType;
    use crate::FieldValue;
    use crate::FromRow;
    use crate::RecordTrait;
    use crate::change;

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
    fn test_update_new_with_change_set() {
        let change_set =
            TestChangeSet { id: change(1), name: change("Updated Name".to_string()), ..Default::default() };
        let update = Update::<TestTable>::new(change_set);
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("UPDATE test_users SET"));
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("id = ?"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_update_many() {
        let update = Update::<TestTable>::many()
            .set(TestColumn::Name, "Anonymous")
            .filter(Condition::is_null(TestColumn::Email));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("UPDATE test_users SET name = ?"));
        assert!(sql.contains("WHERE (email IS NULL)"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_update_set() {
        let update = Update::<TestTable>::many()
            .set(TestColumn::Name, "New Name")
            .set(TestColumn::Email, "new@email.com")
            .filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_update_filter() {
        let update =
            Update::<TestTable>::many().set(TestColumn::Name, "Test").filter(Condition::gt(TestColumn::Id, 10));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("WHERE (id > ?)"));
    }

    #[test]
    fn test_update_multiple_filters() {
        let update = Update::<TestTable>::many()
            .set(TestColumn::Name, "Test")
            .filter(Condition::gt(TestColumn::Id, 10))
            .filter(Condition::is_not_null(TestColumn::Email));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("(id > ?)"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("(email IS NOT NULL)"));
    }

    #[test]
    fn test_update_no_columns_error() {
        let update = Update::<TestTable>::many().filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        assert!(result.is_err());
    }

    #[test]
    fn test_update_change_set_without_pk_error() {
        let change_set = TestChangeSet { name: change("Test".to_string()), ..Default::default() };
        let update = Update::<TestTable>::new(change_set);
        let result = update.build();

        assert!(result.is_err());
    }

    #[test]
    fn test_update_change_set_without_pk_but_with_filter() {
        let change_set = TestChangeSet { name: change("Test".to_string()), ..Default::default() };
        let update = Update::<TestTable>::new(change_set).filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_update_default() {
        let update = Update::<TestTable>::default();

        assert!(format!("{:?}", update).contains("Update"));
    }

    #[test]
    fn test_update_clone() {
        let update = Update::<TestTable>::many().set(TestColumn::Name, "Test").filter(Condition::eq(TestColumn::Id, 1));
        let cloned = update.clone();

        let (sql1, params1) = update.build().unwrap();
        let (sql2, params2) = cloned.build().unwrap();

        assert_eq!(sql1, sql2);
        assert_eq!(params1, params2);
    }

    #[test]
    fn test_update_debug() {
        let update = Update::<TestTable>::many().set(TestColumn::Name, "Test");
        let debug = format!("{:?}", update);
        assert!(debug.contains("Update"));
    }

    #[test]
    fn test_update_change_set_all_fields() {
        let change_set = TestChangeSet {
            id:    change(1),
            name:  change("Alice".to_string()),
            email: change("alice@example.com".to_string()),
        };
        let update = Update::<TestTable>::new(change_set);
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert!(sql.contains("WHERE (id = ?)"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_update_change_set_with_additional_sets() {
        let change_set = TestChangeSet { id: change(1), name: change("Alice".to_string()), ..Default::default() };
        let update = Update::<TestTable>::new(change_set).set(TestColumn::Email, "alice@new.com");
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_update_with_complex_condition() {
        let update = Update::<TestTable>::many()
            .set(TestColumn::Name, "Updated")
            .filter(Condition::eq(TestColumn::Id, 1).and(Condition::is_not_null(TestColumn::Email)));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("AND"));
    }

    #[test]
    fn test_update_with_in_condition() {
        let update = Update::<TestTable>::many()
            .set(TestColumn::Name, "Batch Updated")
            .filter(Condition::is_in(TestColumn::Id, vec![1, 2, 3]));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("id IN (?, ?, ?)"));
        assert_eq!(params.len(), 4);
    }
}
