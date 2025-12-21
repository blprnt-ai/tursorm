use std::collections::HashMap;
use std::marker::PhantomData;

use crate::ColumnTrait;
use crate::Condition;
use crate::FromRow;
use crate::Order;
use crate::OrderBy;
use crate::Result;
use crate::TableTrait;

#[derive(Clone, Debug)]
pub struct Select<Table: TableTrait> {
    conditions: Vec<Condition>,
    order_by:   Vec<OrderBy>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    columns:    Option<Vec<String>>,
    _entity:    PhantomData<Table>,
}

impl<Table: TableTrait> Select<Table> {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            order_by:   Vec::new(),
            limit:      None,
            offset:     None,
            columns:    None,
            _entity:    PhantomData,
        }
    }

    pub fn filter(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn and_filter(self, condition: Condition) -> Self {
        self.filter(condition)
    }

    pub fn columns<Column: ColumnTrait>(mut self, columns: Vec<Column>) -> Self {
        self.columns = Some(columns.iter().map(|c| c.name().to_string()).collect());
        self
    }

    pub fn order_by_asc<Column: ColumnTrait>(mut self, column: Column) -> Self {
        self.order_by.push(OrderBy::asc(column));
        self
    }

    pub fn order_by_desc<Column: ColumnTrait>(mut self, column: Column) -> Self {
        self.order_by.push(OrderBy::desc(column));
        self
    }

    pub fn order_by<Column: ColumnTrait>(mut self, column: Column, direction: Order) -> Self {
        self.order_by.push(OrderBy { column: column.name().to_string(), direction });
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(&self) -> (String, Vec<turso::Value>) {
        let columns = self.columns.as_ref().map(|c| c.join(", ")).unwrap_or_else(|| Table::all_columns().to_string());

        let mut sql = format!("SELECT {} FROM {}", columns, Table::table_name());
        let mut params = Vec::new();

        if !self.conditions.is_empty() {
            let where_parts: Vec<String> = self.conditions.iter().map(|c| format!("({})", c.sql())).collect();
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));

            for condition in &self.conditions {
                params.extend(condition.values().iter().cloned());
            }
        }

        if !self.order_by.is_empty() {
            let order_parts: Vec<String> =
                self.order_by.iter().map(|o| format!("{} {}", o.column, o.direction)).collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_parts.join(", "));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }

    pub async fn all(self, conn: &crate::Connection) -> Result<Vec<Table::Record>> {
        let (sql, params) = self.build();
        let params: Vec<turso::Value> = params.into_iter().collect();

        let mut rows = conn.query(&sql, params).await?;
        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            match Table::Record::from_row(&row) {
                Ok(parsed_row) => results.push(parsed_row),
                Err(e) => {
                    let values = self.from_raw_row(&row)?;

                    #[cfg(feature = "serde")]
                    tracing::warn!("Failed to parse row: {}", serde_json::to_string_pretty(&values)?);

                    #[cfg(not(feature = "serde"))]
                    tracing::warn!("Failed to parse row: {:?}", values);

                    tracing::warn!("{}", e.to_string());

                    continue;
                }
            }
        }

        Ok(results)
    }

    pub async fn one(self, conn: &crate::Connection) -> Result<Option<Table::Record>> {
        let query = self.limit(1);
        let (sql, params) = query.build();
        tracing::trace!("SQL: {}", sql);
        tracing::trace!("Params: {:?}", params);

        let mut rows = conn.query(&sql, params).await?;
        let row = rows.next().await?;
        tracing::trace!("Row: {:?}", row);

        row.map(|r| Table::Record::from_row(&r)).transpose()
    }

    pub async fn count(self, conn: &crate::Connection) -> Result<i64> {
        let mut sql = format!("SELECT COUNT(*) FROM {}", Table::table_name());
        let mut params = Vec::new();

        if !self.conditions.is_empty() {
            let where_parts: Vec<String> = self.conditions.iter().map(|c| format!("({})", c.sql())).collect();
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));

            for condition in &self.conditions {
                params.extend(condition.values().iter().cloned());
            }
        }

        let params: Vec<turso::Value> = params.into_iter().collect();
        let mut rows = conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? {
            let value = row.get_value(0)?;
            match value {
                turso::Value::Integer(count) => Ok(count),
                _ => Ok(0),
            }
        } else {
            Ok(0)
        }
    }

    pub async fn exists(self, conn: &crate::Connection) -> Result<bool> {
        let count = self.limit(1).count(conn).await?;
        Ok(count > 0)
    }

    #[cfg(feature = "serde")]
    fn from_raw_row(&self, row: &turso::Row) -> Result<serde_json::Value> {
        use serde_json::json;

        let column_names = Table::all_columns().split(", ").collect::<Vec<&str>>();
        let column_count = row.column_count();
        let mut values = HashMap::new();

        for i in 0..column_count {
            use serde_json::json;

            let column_name =
                if let Some(column) = &column_names.get(i) { column.to_string() } else { format!("column_{}", i) };
            let value = match row.get_value(i)? {
                turso::Value::Integer(count) => json!(count),
                turso::Value::Real(real) => json!(real),
                turso::Value::Text(text) => match serde_json::from_str(&text) {
                    Ok(value) => value,
                    Err(_) => json!(text),
                },
                turso::Value::Blob(blob) => json!(blob),
                turso::Value::Null => json!(null),
            };

            values.insert(column_name, value);
        }

        Ok(json!(values))
    }

    #[cfg(not(feature = "serde"))]
    fn from_raw_row(&self, row: &turso::Row) -> Result<HashMap<String, String>> {
        let column_names = self.columns.as_ref().cloned().unwrap_or_default();
        let column_count = row.column_count();
        let mut values = HashMap::new();

        for i in 0..column_count {
            let column_name =
                if let Some(column) = &column_names.get(i) { column.to_string() } else { format!("column_{}", i) };
            let value = match row.get_value(i)? {
                turso::Value::Integer(count) => format!("INTEGER: {}", count),
                turso::Value::Real(real) => format!("REAL: {}", real),
                turso::Value::Text(text) => format!("TEXT: {}", text),
                turso::Value::Blob(blob) => format!("BLOB: {:?}", blob),
                turso::Value::Null => format!("NULL"),
            };

            values.insert(column_name, value.to_string());
        }

        Ok(values)
    }
}

impl<Table: TableTrait> Default for Select<Table> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeSetTrait;
    use crate::ColumnType;
    use crate::FieldValue;
    use crate::FromRow;
    use crate::IntoValue;
    use crate::RecordTrait;

    #[derive(Clone, Debug, PartialEq)]
    struct TestRecord {
        id:    i64,
        name:  String,
        email: String,
        age:   Option<i64>,
    }

    impl RecordTrait for TestRecord {
        type Table = TestTable;

        fn get_primary_key_value(&self) -> turso::Value {
            turso::Value::Integer(self.id)
        }
    }

    impl FromRow for TestRecord {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(TestRecord { id: 1, name: "test".to_string(), email: "test@test.com".to_string(), age: Some(25) })
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

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<turso::Value>) {
            let mut columns = Vec::new();
            let mut values = Vec::new();
            if self.name.is_changed() {
                columns.push("name");
                values.push(self.name.clone().take().unwrap().into_value());
            }
            if self.email.is_changed() {
                columns.push("email");
                values.push(self.email.clone().take().unwrap().into_value());
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, turso::Value)> {
            let mut sets = Vec::new();
            if self.name.is_changed() {
                sets.push(("name", self.name.clone().take().unwrap().into_value()));
            }
            if self.email.is_changed() {
                sets.push(("email", self.email.clone().take().unwrap().into_value()));
            }
            sets
        }

        fn get_primary_key_value(&self) -> Option<turso::Value> {
            self.id.clone().take().map(|v| turso::Value::Integer(v))
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

        fn all() -> &'static [Self] {
            &[TestColumn::Id, TestColumn::Name, TestColumn::Email, TestColumn::Age]
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
            "id, name, email, age"
        }

        fn column_count() -> usize {
            4
        }
    }

    #[test]
    fn test_select_new() {
        let select = Select::<TestTable>::new();
        let (sql, params) = select.build();
        assert_eq!(sql, "SELECT id, name, email, age FROM test_users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_select_default() {
        let select = Select::<TestTable>::default();
        let (sql, _) = select.build();
        assert_eq!(sql, "SELECT id, name, email, age FROM test_users");
    }

    #[test]
    fn test_select_filter_single() {
        let select = Select::<TestTable>::new().filter(Condition::eq(TestColumn::Id, 1));
        let (sql, params) = select.build();

        assert_eq!(sql, "SELECT id, name, email, age FROM test_users WHERE (id = ?)");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], turso::Value::Integer(1));
    }

    #[test]
    fn test_select_filter_multiple() {
        let select = Select::<TestTable>::new()
            .filter(Condition::eq(TestColumn::Name, "Alice"))
            .filter(Condition::gt(TestColumn::Age, 18));
        let (sql, params) = select.build();

        assert!(sql.contains("WHERE"));
        assert!(sql.contains("(name = ?)"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("(age > ?)"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_select_and_filter() {
        let select = Select::<TestTable>::new().and_filter(Condition::eq(TestColumn::Id, 1));
        let (sql, _) = select.build();

        assert!(sql.contains("WHERE (id = ?)"));
    }

    #[test]
    fn test_select_specific_columns() {
        let select = Select::<TestTable>::new().columns(vec![TestColumn::Id, TestColumn::Name]);
        let (sql, _) = select.build();

        assert_eq!(sql, "SELECT id, name FROM test_users");
    }

    #[test]
    fn test_select_order_by_asc() {
        let select = Select::<TestTable>::new().order_by_asc(TestColumn::Name);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY name ASC"));
    }

    #[test]
    fn test_select_order_by_desc() {
        let select = Select::<TestTable>::new().order_by_desc(TestColumn::Age);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY age DESC"));
    }

    #[test]
    fn test_select_order_by_with_direction() {
        let select = Select::<TestTable>::new().order_by(TestColumn::Id, Order::Desc);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY id DESC"));
    }

    #[test]
    fn test_select_multiple_order_by() {
        let select = Select::<TestTable>::new().order_by_asc(TestColumn::Name).order_by_desc(TestColumn::Age);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY name ASC, age DESC"));
    }

    #[test]
    fn test_select_limit() {
        let select = Select::<TestTable>::new().limit(10);
        let (sql, _) = select.build();

        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_select_offset() {
        let select = Select::<TestTable>::new().offset(20);
        let (sql, _) = select.build();

        assert!(sql.contains("OFFSET 20"));
    }

    #[test]
    fn test_select_limit_and_offset() {
        let select = Select::<TestTable>::new().limit(10).offset(20);
        let (sql, _) = select.build();

        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 20"));
    }

    #[test]
    fn test_select_complex_query() {
        let select = Select::<TestTable>::new()
            .filter(Condition::eq(TestColumn::Name, "Alice"))
            .filter(Condition::is_not_null(TestColumn::Email))
            .order_by_desc(TestColumn::Age)
            .limit(10)
            .offset(0);
        let (sql, params) = select.build();

        assert!(sql.contains("SELECT id, name, email, age FROM test_users"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("(name = ?)"));
        assert!(sql.contains("(email IS NOT NULL)"));
        assert!(sql.contains("ORDER BY age DESC"));
        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 0"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_select_clause_order() {
        let select = Select::<TestTable>::new()
            .limit(5)
            .filter(Condition::eq(TestColumn::Id, 1))
            .offset(10)
            .order_by_asc(TestColumn::Name);
        let (sql, _) = select.build();

        let where_pos = sql.find("WHERE").unwrap();
        let order_pos = sql.find("ORDER BY").unwrap();
        let limit_pos = sql.find("LIMIT").unwrap();
        let offset_pos = sql.find("OFFSET").unwrap();

        assert!(where_pos < order_pos);
        assert!(order_pos < limit_pos);
        assert!(limit_pos < offset_pos);
    }

    #[test]
    fn test_select_clone() {
        let select = Select::<TestTable>::new().filter(Condition::eq(TestColumn::Id, 1)).limit(10);
        let cloned = select.clone();

        let (sql1, params1) = select.build();
        let (sql2, params2) = cloned.build();

        assert_eq!(sql1, sql2);
        assert_eq!(params1, params2);
    }

    #[test]
    fn test_select_debug() {
        let select = Select::<TestTable>::new().limit(5);
        let debug = format!("{:?}", select);
        assert!(debug.contains("Select"));
    }

    #[test]
    fn test_select_no_conditions() {
        let select = Select::<TestTable>::new();
        let (sql, params) = select.build();

        assert!(!sql.contains("WHERE"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_select_with_in_condition() {
        let select = Select::<TestTable>::new().filter(Condition::is_in(TestColumn::Id, vec![1, 2, 3]));
        let (sql, params) = select.build();

        assert!(sql.contains("id IN (?, ?, ?)"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_select_with_between_condition() {
        let select = Select::<TestTable>::new().filter(Condition::between(TestColumn::Age, 18, 65));
        let (sql, params) = select.build();

        assert!(sql.contains("age BETWEEN ? AND ?"));
        assert_eq!(params.len(), 2);
    }
}
