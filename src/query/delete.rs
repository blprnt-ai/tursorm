//! DELETE query builder

use std::marker::PhantomData;

use crate::ModelTrait;
use crate::entity::EntityTrait;
use crate::error::Result;
use crate::query::condition::Condition;
use crate::value::Value;

/// DELETE query builder for removing records from the database
///
/// Use this builder to delete records with optional filtering conditions.
/// Without any filters, it will delete all records (use with caution!).
///
/// # Example
///
/// ```ignore
/// // Delete by ID
/// Delete::<UserEntity>::new()
///     .filter(Condition::eq(UserColumn::Id, 1))
///     .exec(&conn)
///     .await?;
///
/// // Delete all inactive users
/// Delete::<UserEntity>::new()
///     .filter(Condition::eq(UserColumn::Status, "inactive"))
///     .exec(&conn)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct Delete<E: EntityTrait> {
    conditions: Vec<Condition>,
    _entity:    PhantomData<E>,
}

impl<E: EntityTrait> Delete<E> {
    /// Create a new DELETE query
    pub fn new() -> Self {
        Self { conditions: Vec::new(), _entity: PhantomData }
    }

    /// Add a filter condition
    pub fn filter(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("DELETE FROM {}", E::table_name());
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

        (sql, params)
    }

    /// Execute the delete and return the number of rows affected
    ///
    /// # Warning
    ///
    /// If no filter conditions are set, this will delete ALL rows in the table!
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution fails.
    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
        let (sql, params) = self.build();
        let params: Vec<turso::Value> = params.into_iter().collect();
        let affected = conn.execute(&sql, params).await?;
        Ok(affected)
    }
}

impl<E: EntityTrait> Default for Delete<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Delete a model by its primary key
///
/// Convenience function to delete a record using its model instance.
/// The primary key value is extracted from the model automatically.
///
/// # Example
///
/// ```ignore
/// let user = User::find_by_id(1).one(&conn).await?.unwrap();
/// delete_by_model::<UserEntity>(&conn, &user).await?;
/// ```
pub async fn delete_by_model<E: EntityTrait>(conn: &crate::Connection, model: &E::Model) -> Result<u64>
where E::Model: ModelTrait {
    let pk_value = model.get_primary_key_value();
    let pk_column = E::primary_key();

    Delete::<E>::new().filter(Condition::eq(pk_column, pk_value)).exec(conn).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ActiveModelTrait;
    use crate::entity::ActiveValue;
    use crate::entity::ColumnTrait;
    use crate::entity::FromRow;
    use crate::value::ColumnType;
    use crate::value::Value;

    // Mock Entity and related types for testing
    #[derive(Clone, Debug, PartialEq)]
    struct TestModel {
        id:    i64,
        name:  String,
        email: String,
    }

    impl ModelTrait for TestModel {
        type Entity = TestEntity;

        fn get_primary_key_value(&self) -> Value {
            Value::Integer(self.id)
        }
    }

    impl FromRow for TestModel {
        fn from_row(_row: &turso::Row) -> crate::error::Result<Self> {
            Ok(TestModel { id: 1, name: "test".to_string(), email: "test@test.com".to_string() })
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
                values.push(Value::Text(self.name.clone().take().unwrap()));
            }
            if self.email.is_set() {
                columns.push("email");
                values.push(Value::Text(self.email.clone().take().unwrap()));
            }
            (columns, values)
        }

        fn get_update_sets(&self) -> Vec<(&'static str, Value)> {
            let mut sets = Vec::new();
            if self.name.is_set() {
                sets.push(("name", Value::Text(self.name.clone().take().unwrap())));
            }
            if self.email.is_set() {
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
            "id, name, email"
        }

        fn column_count() -> usize {
            3
        }
    }

    // Delete::new tests
    #[test]
    fn test_delete_new() {
        let delete = Delete::<TestEntity>::new();
        let (sql, params) = delete.build();

        assert_eq!(sql, "DELETE FROM test_users");
        assert!(params.is_empty());
    }

    // Delete::default tests
    #[test]
    fn test_delete_default() {
        let delete = Delete::<TestEntity>::default();
        let (sql, params) = delete.build();

        assert_eq!(sql, "DELETE FROM test_users");
        assert!(params.is_empty());
    }

    // Delete::filter tests
    #[test]
    fn test_delete_filter_single() {
        let delete = Delete::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1));
        let (sql, params) = delete.build();

        assert_eq!(sql, "DELETE FROM test_users WHERE (id = ?)");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Integer(1));
    }

    #[test]
    fn test_delete_filter_multiple() {
        let delete = Delete::<TestEntity>::new()
            .filter(Condition::eq(TestColumn::Name, "Alice"))
            .filter(Condition::is_null(TestColumn::Email));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE"));
        assert!(sql.contains("(name = ?)"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("(email IS NULL)"));
        assert_eq!(params.len(), 1); // Only name has a parameter
    }

    // Test with various conditions
    #[test]
    fn test_delete_with_gt_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::gt(TestColumn::Id, 100));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (id > ?)"));
        assert_eq!(params[0], Value::Integer(100));
    }

    #[test]
    fn test_delete_with_lt_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::lt(TestColumn::Id, 10));
        let (sql, _) = delete.build();

        assert!(sql.contains("WHERE (id < ?)"));
    }

    #[test]
    fn test_delete_with_like_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::like(TestColumn::Name, "%test%"));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (name LIKE ?)"));
        assert_eq!(params[0], Value::Text("%test%".to_string()));
    }

    #[test]
    fn test_delete_with_is_null_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::is_null(TestColumn::Email));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (email IS NULL)"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_delete_with_is_not_null_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::is_not_null(TestColumn::Email));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (email IS NOT NULL)"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_delete_with_in_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::is_in(TestColumn::Id, vec![1, 2, 3, 4, 5]));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (id IN (?, ?, ?, ?, ?))"));
        assert_eq!(params.len(), 5);
    }

    #[test]
    fn test_delete_with_not_in_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::not_in(TestColumn::Id, vec![1, 2]));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (id NOT IN (?, ?))"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_delete_with_between_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::between(TestColumn::Id, 10, 100));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (id BETWEEN ? AND ?)"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], Value::Integer(10));
        assert_eq!(params[1], Value::Integer(100));
    }

    // Test with combined conditions
    #[test]
    fn test_delete_with_and_condition() {
        let combined =
            Condition::eq(TestColumn::Name, "Alice").and(Condition::eq(TestColumn::Email, "alice@example.com"));
        let delete = Delete::<TestEntity>::new().filter(combined);
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE ((name = ?) AND (email = ?))"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_delete_with_or_condition() {
        let combined = Condition::eq(TestColumn::Name, "Alice").or(Condition::eq(TestColumn::Name, "Bob"));
        let delete = Delete::<TestEntity>::new().filter(combined);
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE ((name = ?) OR (name = ?))"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_delete_with_not_condition() {
        let delete = Delete::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1).not());
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (NOT (id = ?))"));
        assert_eq!(params.len(), 1);
    }

    // Test complex query
    #[test]
    fn test_delete_complex_query() {
        let delete = Delete::<TestEntity>::new()
            .filter(Condition::gt(TestColumn::Id, 0))
            .filter(Condition::contains(TestColumn::Email, "@example.com"))
            .filter(Condition::is_not_null(TestColumn::Name));
        let (sql, params) = delete.build();

        assert!(sql.contains("DELETE FROM test_users WHERE"));
        assert!(sql.contains("(id > ?)"));
        assert!(sql.contains("(email LIKE ?)"));
        assert!(sql.contains("(name IS NOT NULL)"));
        assert_eq!(params.len(), 2); // id and email pattern
    }

    // Clone tests
    #[test]
    fn test_delete_clone() {
        let delete = Delete::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1));
        let cloned = delete.clone();

        let (sql1, params1) = delete.build();
        let (sql2, params2) = cloned.build();

        assert_eq!(sql1, sql2);
        assert_eq!(params1, params2);
    }

    // Debug tests
    #[test]
    fn test_delete_debug() {
        let delete = Delete::<TestEntity>::new().filter(Condition::eq(TestColumn::Id, 1));
        let debug = format!("{:?}", delete);

        assert!(debug.contains("Delete"));
    }

    // Test no WHERE clause (delete all)
    #[test]
    fn test_delete_all() {
        let delete = Delete::<TestEntity>::new();
        let (sql, params) = delete.build();

        assert_eq!(sql, "DELETE FROM test_users");
        assert!(params.is_empty());
        assert!(!sql.contains("WHERE"));
    }

    // Test chained filters
    #[test]
    fn test_delete_chained_filters() {
        let delete = Delete::<TestEntity>::new()
            .filter(Condition::eq(TestColumn::Id, 1))
            .filter(Condition::eq(TestColumn::Name, "Test"))
            .filter(Condition::is_not_null(TestColumn::Email));
        let (sql, _) = delete.build();

        // Verify all conditions are present
        let where_count = sql.matches("AND").count();
        assert_eq!(where_count, 2); // 3 conditions means 2 ANDs
    }

    // Test with raw condition
    #[test]
    fn test_delete_with_raw_condition() {
        let delete = Delete::<TestEntity>::new()
            .filter(Condition::raw("id > ? AND id < ?", vec![Value::Integer(5), Value::Integer(10)]));
        let (sql, params) = delete.build();

        assert!(sql.contains("WHERE (id > ? AND id < ?)"));
        assert_eq!(params.len(), 2);
    }
}
