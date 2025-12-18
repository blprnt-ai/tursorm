//! UPDATE query builder

use std::marker::PhantomData;

use crate::entity::ActiveModelTrait;
use crate::entity::ColumnTrait;
use crate::entity::EntityTrait;
use crate::entity::FromRow;
use crate::error::Error;
use crate::error::Result;
use crate::query::condition::Condition;
use crate::value::IntoValue;
use crate::value::Value;

/// UPDATE query builder for modifying existing records
///
/// Use this builder to update records. Can update a single record by primary
/// key using an active model, or update multiple records with conditions.
///
/// # Example
///
/// ```ignore
/// // Update a single record using an active model
/// let mut update = UserActiveModel::from(user);
/// update.name = set("New Name".to_string());
/// Update::<UserEntity>::new(update).exec(&conn).await?;
///
/// // Bulk update with conditions
/// Update::<UserEntity>::many()
///     .set(UserColumn::Status, "inactive")
///     .filter(Condition::lt(UserColumn::LastLogin, "2020-01-01"))
///     .exec(&conn)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct Update<E: EntityTrait> {
    model:      Option<E::ActiveModel>,
    sets:       Vec<(String, Value)>,
    conditions: Vec<Condition>,
    _entity:    PhantomData<E>,
}

impl<E: EntityTrait> Update<E> {
    /// Create a new UPDATE query with an active model
    ///
    /// The model's primary key will be used in the WHERE clause, and
    /// all set fields will be updated.
    pub fn new(model: E::ActiveModel) -> Self {
        Self { model: Some(model), sets: Vec::new(), conditions: Vec::new(), _entity: PhantomData }
    }

    /// Create an UPDATE query for bulk updates
    ///
    /// Use this with [`set`](Self::set) and [`filter`](Self::filter) to
    /// update multiple rows matching certain conditions.
    pub fn many() -> Self {
        Self { model: None, sets: Vec::new(), conditions: Vec::new(), _entity: PhantomData }
    }

    /// Set a column value
    pub fn set<C: ColumnTrait, V: IntoValue>(mut self, column: C, value: V) -> Self {
        self.sets.push((column.name().to_string(), value.into_value()));
        self
    }

    /// Add a filter condition
    pub fn filter(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Build the SQL query and parameters
    fn build(&self) -> Result<(String, Vec<Value>)> {
        let mut set_parts = Vec::new();
        let mut params = Vec::new();

        // Get sets from model if provided
        if let Some(ref model) = self.model {
            let model_sets = model.get_update_sets();
            for (col, val) in model_sets {
                set_parts.push(format!("{} = ?", col));
                params.push(val);
            }
        }

        // Add explicit sets
        for (col, val) in &self.sets {
            set_parts.push(format!("{} = ?", col));
            params.push(val.clone());
        }

        if set_parts.is_empty() {
            return Err(Error::Query("No columns to update".to_string()));
        }

        let mut sql = format!("UPDATE {} SET {}", E::table_name(), set_parts.join(", "));

        // Determine WHERE clause
        let mut where_conditions = self.conditions.clone();

        // If we have a model, add primary key condition
        if let Some(ref model) = self.model {
            if let Some(pk_value) = model.get_primary_key_value() {
                let pk_column = E::ActiveModel::primary_key_column();
                where_conditions.push(Condition::raw(format!("{} = ?", pk_column), vec![pk_value]));
            } else if self.conditions.is_empty() {
                return Err(Error::PrimaryKeyNotSet);
            }
        }

        // WHERE clause
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

    /// Execute the update and return the number of rows affected
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No columns are set to update
    /// - The model has no primary key set (when using `new()` without filters)
    /// - The query execution fails
    pub async fn exec(self, conn: &turso::Connection) -> Result<u64> {
        let (sql, params) = self.build()?;
        let params: Vec<turso::Value> = params.into_iter().collect();
        let affected = conn.execute(&sql, params).await?;
        Ok(affected)
    }

    /// Execute the update and return the updated row
    ///
    /// Uses the SQL `RETURNING` clause to get the complete updated row back.
    /// Useful when you need the row's current state after the update.
    ///
    /// Note: Only returns one row. If multiple rows are updated, only the
    /// first one is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if no rows were affected or if the query fails.
    pub async fn exec_with_returning(self, conn: &turso::Connection) -> Result<E::Model> {
        let (base_sql, params) = self.build()?;
        let sql = format!("{} RETURNING {}", base_sql, E::all_columns());

        let params: Vec<turso::Value> = params.into_iter().collect();
        let mut rows = conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? { E::Model::from_row(&row) } else { Err(Error::NoRowsAffected) }
    }
}

impl<E: EntityTrait> Default for Update<E> {
    fn default() -> Self {
        Self::many()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ActiveValue;
    use crate::entity::FromRow;
    use crate::entity::ModelTrait;
    use crate::entity::set;
    use crate::value::ColumnType;

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

    // Update::new tests
    #[test]
    fn test_update_new_with_model() {
        let model = TestActiveModel { id: set(1), name: set("Updated Name".to_string()), ..Default::default() };
        let update = Update::<TestEntity>::new(model);
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("UPDATE test_users SET"));
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("id = ?"));
        assert_eq!(params.len(), 2); // name value + id value
    }

    // Update::many tests
    #[test]
    fn test_update_many() {
        let update = Update::<TestEntity>::many()
            .set(TestColumn::Name, "Anonymous")
            .filter(Condition::is_null(TestColumn::Email));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("UPDATE test_users SET name = ?"));
        assert!(sql.contains("WHERE (email IS NULL)"));
        assert_eq!(params.len(), 1);
    }

    // Update::set tests
    #[test]
    fn test_update_set() {
        let update = Update::<TestEntity>::many()
            .set(TestColumn::Name, "New Name")
            .set(TestColumn::Email, "new@email.com")
            .filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert_eq!(params.len(), 3); // name + email + id
    }

    // Update::filter tests
    #[test]
    fn test_update_filter() {
        let update =
            Update::<TestEntity>::many().set(TestColumn::Name, "Test").filter(Condition::gt(TestColumn::Id, 10));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("WHERE (id > ?)"));
    }

    #[test]
    fn test_update_multiple_filters() {
        let update = Update::<TestEntity>::many()
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

    // Test build errors
    #[test]
    fn test_update_no_columns_error() {
        let update = Update::<TestEntity>::many().filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        assert!(result.is_err());
    }

    #[test]
    fn test_update_model_without_pk_error() {
        let model = TestActiveModel {
            name: set("Test".to_string()),
            // id is not set
            ..Default::default()
        };
        let update = Update::<TestEntity>::new(model);
        let result = update.build();

        assert!(result.is_err());
    }

    #[test]
    fn test_update_model_without_pk_but_with_filter() {
        let model = TestActiveModel {
            name: set("Test".to_string()),
            // id is not set, but we have a filter
            ..Default::default()
        };
        let update = Update::<TestEntity>::new(model).filter(Condition::eq(TestColumn::Id, 1));
        let result = update.build();

        // Should succeed because we have a filter condition
        assert!(result.is_ok());
    }

    // Default tests
    #[test]
    fn test_update_default() {
        let update = Update::<TestEntity>::default();
        // Default is same as many()
        assert!(format!("{:?}", update).contains("Update"));
    }

    // Clone tests
    #[test]
    fn test_update_clone() {
        let update =
            Update::<TestEntity>::many().set(TestColumn::Name, "Test").filter(Condition::eq(TestColumn::Id, 1));
        let cloned = update.clone();

        let (sql1, params1) = update.build().unwrap();
        let (sql2, params2) = cloned.build().unwrap();

        assert_eq!(sql1, sql2);
        assert_eq!(params1, params2);
    }

    // Debug tests
    #[test]
    fn test_update_debug() {
        let update = Update::<TestEntity>::many().set(TestColumn::Name, "Test");
        let debug = format!("{:?}", update);
        assert!(debug.contains("Update"));
    }

    // Test with model that has all fields set
    #[test]
    fn test_update_model_all_fields() {
        let model = TestActiveModel {
            id:    set(1),
            name:  set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
        };
        let update = Update::<TestEntity>::new(model);
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert!(sql.contains("WHERE (id = ?)"));
        assert_eq!(params.len(), 3);
    }

    // Test combining model with additional sets
    #[test]
    fn test_update_model_with_additional_sets() {
        let model = TestActiveModel { id: set(1), name: set("Alice".to_string()), ..Default::default() };
        let update = Update::<TestEntity>::new(model).set(TestColumn::Email, "alice@new.com");
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("name = ?"));
        assert!(sql.contains("email = ?"));
        assert_eq!(params.len(), 3);
    }

    // Test with complex conditions
    #[test]
    fn test_update_with_complex_condition() {
        let update = Update::<TestEntity>::many()
            .set(TestColumn::Name, "Updated")
            .filter(Condition::eq(TestColumn::Id, 1).and(Condition::is_not_null(TestColumn::Email)));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("AND"));
    }

    // Test with IN condition
    #[test]
    fn test_update_with_in_condition() {
        let update = Update::<TestEntity>::many()
            .set(TestColumn::Name, "Batch Updated")
            .filter(Condition::is_in(TestColumn::Id, vec![1, 2, 3]));
        let result = update.build();

        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("id IN (?, ?, ?)"));
        assert_eq!(params.len(), 4); // 1 for set + 3 for IN
    }
}
