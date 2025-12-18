//! SELECT query builder

use std::marker::PhantomData;

use crate::ColumnTrait;
use crate::Condition;
use crate::EntityTrait;
use crate::FromRow;
use crate::ModelTrait;
use crate::Order;
use crate::OrderBy;
use crate::Result;
use crate::Value;

/// SELECT query builder for retrieving records from the database
///
/// Use this builder to construct SELECT queries with filtering, ordering,
/// and pagination. The builder uses a fluent API for chaining operations.
///
/// # Example
///
/// ```ignore
/// let users = Select::<UserEntity>::new()
///     .filter(Condition::eq(UserColumn::Status, "active"))
///     .order_by_desc(UserColumn::CreatedAt)
///     .limit(10)
///     .all(&conn)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct Select<E: EntityTrait> {
    conditions: Vec<Condition>,
    order_by:   Vec<OrderBy>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    columns:    Option<Vec<String>>,
    _entity:    PhantomData<E>,
}

impl<E: EntityTrait> Select<E> {
    /// Create a new SELECT query
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

    /// Add a filter condition
    pub fn filter(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add an AND filter (alias for filter)
    pub fn and_filter(self, condition: Condition) -> Self {
        self.filter(condition)
    }

    /// Select specific columns
    pub fn columns<C: ColumnTrait>(mut self, columns: Vec<C>) -> Self {
        self.columns = Some(columns.iter().map(|c| c.name().to_string()).collect());
        self
    }

    /// Add ORDER BY clause (ascending)
    pub fn order_by_asc<C: ColumnTrait>(mut self, column: C) -> Self {
        self.order_by.push(OrderBy::asc(column));
        self
    }

    /// Add ORDER BY clause (descending)
    pub fn order_by_desc<C: ColumnTrait>(mut self, column: C) -> Self {
        self.order_by.push(OrderBy::desc(column));
        self
    }

    /// Add ORDER BY clause with direction
    pub fn order_by<C: ColumnTrait>(mut self, column: C, direction: Order) -> Self {
        self.order_by.push(OrderBy { column: column.name().to_string(), direction });
        self
    }

    /// Set LIMIT clause
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set OFFSET clause
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let columns = self.columns.as_ref().map(|c| c.join(", ")).unwrap_or_else(|| E::all_columns().to_string());

        let mut sql = format!("SELECT {} FROM {}", columns, E::table_name());
        let mut params = Vec::new();

        // WHERE clause
        if !self.conditions.is_empty() {
            let where_parts: Vec<String> = self.conditions.iter().map(|c| format!("({})", c.sql())).collect();
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));

            for condition in &self.conditions {
                params.extend(condition.values().iter().cloned());
            }
        }

        // ORDER BY clause
        if !self.order_by.is_empty() {
            let order_parts: Vec<String> =
                self.order_by.iter().map(|o| format!("{} {}", o.column, o.direction)).collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_parts.join(", "));
        }

        // LIMIT clause
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // OFFSET clause
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }

    /// Execute the query and return all matching results
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution fails or if any row
    /// cannot be converted to the model type.
    pub async fn all(self, conn: &crate::Connection) -> Result<Vec<E::Model>> {
        let (sql, params) = self.build();
        let params: Vec<turso::Value> = params.into_iter().collect();

        let mut rows = conn.query(&sql, params).await?;
        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            results.push(E::Model::from_row(&row)?);
        }

        Ok(results)
    }

    /// Execute the query and return the first result
    ///
    /// Automatically applies `LIMIT 1` to the query. Returns `None` if
    /// no rows match the query conditions.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution fails or if the row
    /// cannot be converted to the model type.
    pub async fn one(self, conn: &crate::Connection) -> Result<Option<E::Model>> {
        let query = self.limit(1);
        let (sql, params) = query.build();
        let params: Vec<turso::Value> = params.into_iter().collect();

        let mut rows = conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? { Ok(Some(E::Model::from_row(&row)?)) } else { Ok(None) }
    }

    /// Execute a COUNT(*) query and return the number of matching rows
    ///
    /// This method ignores any ORDER BY, LIMIT, or OFFSET clauses and
    /// only uses the WHERE conditions.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution fails.
    pub async fn count(self, conn: &crate::Connection) -> Result<i64> {
        let mut sql = format!("SELECT COUNT(*) FROM {}", E::table_name());
        let mut params = Vec::new();

        // WHERE clause
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
                Value::Integer(count) => Ok(count),
                _ => Ok(0),
            }
        } else {
            Ok(0)
        }
    }

    /// Check if any rows exist matching the query conditions
    ///
    /// This is more efficient than `count()` when you only need to know
    /// if at least one row exists, as it applies `LIMIT 1`.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution fails.
    pub async fn exists(self, conn: &crate::Connection) -> Result<bool> {
        let count = self.limit(1).count(conn).await?;
        Ok(count > 0)
    }
}

impl<E: EntityTrait> Default for Select<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for models to enable fluent querying
///
/// This trait is automatically implemented for all types that implement
/// [`ModelTrait`], providing convenient static methods for querying.
///
/// # Example
///
/// ```ignore
/// // Find all users
/// let users = User::find().all(&conn).await?;
///
/// // Find a user by ID
/// let user = User::find_by_id(1).one(&conn).await?;
/// ```
pub trait SelectExt: ModelTrait {
    /// Create a SELECT query for all records
    fn find() -> Select<Self::Entity> {
        Select::new()
    }

    /// Create a SELECT query filtered by primary key
    fn find_by_id<V: crate::value::IntoValue>(id: V) -> Select<Self::Entity>
    where <Self::Entity as EntityTrait>::Column: ColumnTrait {
        Select::new().filter(Condition::eq(<Self::Entity as EntityTrait>::primary_key(), id))
    }
}

impl<M: ModelTrait> SelectExt for M {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActiveModelTrait;
    use crate::ActiveValue;
    use crate::ColumnType;
    use crate::FromRow;
    use crate::IntoValue;

    // Mock Entity and related types for testing
    #[derive(Clone, Debug, PartialEq)]
    struct TestModel {
        id:    i64,
        name:  String,
        email: String,
        age:   Option<i64>,
    }

    impl ModelTrait for TestModel {
        type Entity = TestEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for TestModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            // Mock implementation - in real tests this would parse the row
            Ok(TestModel { id: 1, name: "test".to_string(), email: "test@test.com".to_string(), age: Some(25) })
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestActiveModel {
        id:    ActiveValue<i64>,
        name:  ActiveValue<String>,
        email: ActiveValue<String>,
    }

    impl ActiveModelTrait for TestActiveModel {
        type Entity = TestEntity;

        fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>) {
            let mut columns = Vec::new();
            let mut values = Vec::new();
            if self.name.is_set() {
                columns.push("name");
                values.push(self.name.clone().take().unwrap().into_value());
            }
            if self.email.is_set() {
                columns.push("email");
                values.push(self.email.clone().take().unwrap().into_value());
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            let mut sets = Vec::new();
            if self.name.is_set() {
                sets.push(("name", self.name.clone().take().unwrap().into_value()));
            }
            if self.email.is_set() {
                sets.push(("email", self.email.clone().take().unwrap().into_value()));
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

        fn all() -> &'static [Self] {
            &[TestColumn::Id, TestColumn::Name, TestColumn::Email, TestColumn::Age]
        }
    }

    #[derive(Default, Clone, Debug)]
    struct TestEntity;

    impl EntityTrait for TestEntity {
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

    // Select::new tests
    #[test]
    fn test_select_new() {
        let select = Select::<TestEntity>::new();
        let (sql, params) = select.build();
        assert_eq!(sql, "SELECT id, name, email, age FROM test_users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_select_default() {
        let select = Select::<TestEntity>::default();
        let (sql, _) = select.build();
        assert_eq!(sql, "SELECT id, name, email, age FROM test_users");
    }

    // Select::filter tests
    #[test]
    fn test_select_filter_single() {
        let select = Select::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1));
        let (sql, params) = select.build();

        assert_eq!(sql, "SELECT id, name, email, age FROM test_users WHERE (id = ?)");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Integer(1));
    }

    #[test]
    fn test_select_filter_multiple() {
        let select = Select::<TestEntity>::new()
            .filter(Condition::eq(TestColumn::Name, "Alice"))
            .filter(Condition::gt(TestColumn::Age, 18));
        let (sql, params) = select.build();

        assert!(sql.contains("WHERE"));
        assert!(sql.contains("(name = ?)"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("(age > ?)"));
        assert_eq!(params.len(), 2);
    }

    // Select::and_filter tests
    #[test]
    fn test_select_and_filter() {
        let select = Select::<TestEntity>::new().and_filter(Condition::eq(TestColumn::Id, 1));
        let (sql, _) = select.build();

        assert!(sql.contains("WHERE (id = ?)"));
    }

    // Select::columns tests
    #[test]
    fn test_select_specific_columns() {
        let select = Select::<TestEntity>::new().columns(vec![TestColumn::Id, TestColumn::Name]);
        let (sql, _) = select.build();

        assert_eq!(sql, "SELECT id, name FROM test_users");
    }

    // Select::order_by tests
    #[test]
    fn test_select_order_by_asc() {
        let select = Select::<TestEntity>::new().order_by_asc(TestColumn::Name);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY name ASC"));
    }

    #[test]
    fn test_select_order_by_desc() {
        let select = Select::<TestEntity>::new().order_by_desc(TestColumn::Age);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY age DESC"));
    }

    #[test]
    fn test_select_order_by_with_direction() {
        let select = Select::<TestEntity>::new().order_by(TestColumn::Id, Order::Desc);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY id DESC"));
    }

    #[test]
    fn test_select_multiple_order_by() {
        let select = Select::<TestEntity>::new().order_by_asc(TestColumn::Name).order_by_desc(TestColumn::Age);
        let (sql, _) = select.build();

        assert!(sql.contains("ORDER BY name ASC, age DESC"));
    }

    // Select::limit tests
    #[test]
    fn test_select_limit() {
        let select = Select::<TestEntity>::new().limit(10);
        let (sql, _) = select.build();

        assert!(sql.contains("LIMIT 10"));
    }

    // Select::offset tests
    #[test]
    fn test_select_offset() {
        let select = Select::<TestEntity>::new().offset(20);
        let (sql, _) = select.build();

        assert!(sql.contains("OFFSET 20"));
    }

    #[test]
    fn test_select_limit_and_offset() {
        let select = Select::<TestEntity>::new().limit(10).offset(20);
        let (sql, _) = select.build();

        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 20"));
    }

    // Combined query tests
    #[test]
    fn test_select_complex_query() {
        let select = Select::<TestEntity>::new()
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
        assert_eq!(params.len(), 1); // Only name = ? has a value
    }

    // Test query building order
    #[test]
    fn test_select_clause_order() {
        let select = Select::<TestEntity>::new()
            .limit(5)
            .filter(Condition::eq(TestColumn::Id, 1))
            .offset(10)
            .order_by_asc(TestColumn::Name);
        let (sql, _) = select.build();

        // Verify clauses are in correct order regardless of method call order
        let where_pos = sql.find("WHERE").unwrap();
        let order_pos = sql.find("ORDER BY").unwrap();
        let limit_pos = sql.find("LIMIT").unwrap();
        let offset_pos = sql.find("OFFSET").unwrap();

        assert!(where_pos < order_pos);
        assert!(order_pos < limit_pos);
        assert!(limit_pos < offset_pos);
    }

    // Clone tests
    #[test]
    fn test_select_clone() {
        let select = Select::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1)).limit(10);
        let cloned = select.clone();

        let (sql1, params1) = select.build();
        let (sql2, params2) = cloned.build();

        assert_eq!(sql1, sql2);
        assert_eq!(params1, params2);
    }

    // Debug test
    #[test]
    fn test_select_debug() {
        let select = Select::<TestEntity>::new().limit(5);
        let debug = format!("{:?}", select);
        assert!(debug.contains("Select"));
    }

    // Empty conditions test
    #[test]
    fn test_select_no_conditions() {
        let select = Select::<TestEntity>::new();
        let (sql, params) = select.build();

        assert!(!sql.contains("WHERE"));
        assert!(params.is_empty());
    }

    // Test with IN condition
    #[test]
    fn test_select_with_in_condition() {
        let select = Select::<TestEntity>::new().filter(Condition::is_in(TestColumn::Id, vec![1, 2, 3]));
        let (sql, params) = select.build();

        assert!(sql.contains("id IN (?, ?, ?)"));
        assert_eq!(params.len(), 3);
    }

    // Test with BETWEEN condition
    #[test]
    fn test_select_with_between_condition() {
        let select = Select::<TestEntity>::new().filter(Condition::between(TestColumn::Age, 18, 65));
        let (sql, params) = select.build();

        assert!(sql.contains("age BETWEEN ? AND ?"));
        assert_eq!(params.len(), 2);
    }
}
