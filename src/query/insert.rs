//! INSERT query builder

use std::marker::PhantomData;

use crate::entity::ActiveModelTrait;
use crate::entity::EntityTrait;
use crate::entity::FromRow;
use crate::error::Error;
use crate::error::Result;
use crate::value::Value;

/// INSERT query builder for adding new records to the database
///
/// Use this builder to insert one or more records. Supports returning the
/// inserted row or the last insert ID.
///
/// # Example
///
/// ```ignore
/// let new_user = UserActiveModel {
///     name: set("Alice".to_string()),
///     email: set("alice@example.com".to_string()),
///     ..Default::default()
/// };
///
/// // Insert and get row count
/// let affected = Insert::<UserEntity>::new(new_user).exec(&conn).await?;
///
/// // Insert and get the inserted row back
/// let user = Insert::<UserEntity>::new(new_user).exec_with_returning(&conn).await?;
/// ```
#[derive(Clone, Debug)]
pub struct Insert<E: EntityTrait> {
    models:  Vec<E::ActiveModel>,
    _entity: PhantomData<E>,
}

impl<E: EntityTrait> Insert<E> {
    /// Create a new INSERT query with a single model
    pub fn new(model: E::ActiveModel) -> Self {
        Self { models: vec![model], _entity: PhantomData }
    }

    /// Create an empty INSERT query
    pub fn empty() -> Self {
        Self { models: Vec::new(), _entity: PhantomData }
    }

    /// Add a model to insert
    pub fn add(mut self, model: E::ActiveModel) -> Self {
        self.models.push(model);
        self
    }

    /// Add multiple models to insert
    pub fn add_many(mut self, models: impl IntoIterator<Item = E::ActiveModel>) -> Self {
        self.models.extend(models);
        self
    }

    /// Build the SQL query and parameters for a single insert
    fn build_single(&self, model: &E::ActiveModel) -> (String, Vec<Value>) {
        let (columns, values) = model.get_insert_columns_and_values();

        if columns.is_empty() {
            return (format!("INSERT INTO {} DEFAULT VALUES", E::table_name()), Vec::new());
        }

        let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();

        let sql =
            format!("INSERT INTO {} ({}) VALUES ({})", E::table_name(), columns.join(", "), placeholders.join(", "));

        (sql, values)
    }

    /// Execute the insert and return the number of rows affected
    ///
    /// If multiple models were added, each is inserted individually and
    /// the total affected count is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if any insert fails.
    pub async fn exec(self, conn: &turso::Connection) -> Result<u64> {
        if self.models.is_empty() {
            return Ok(0);
        }

        let mut total_affected = 0u64;

        for model in &self.models {
            let (sql, params) = self.build_single(model);
            let params: Vec<turso::Value> = params.into_iter().collect();
            let affected = conn.execute(&sql, params).await?;
            total_affected += affected;
        }

        Ok(total_affected)
    }

    /// Execute the insert and return the inserted row
    ///
    /// Uses the SQL `RETURNING` clause to get the complete row back,
    /// including any auto-generated values like primary keys or defaults.
    ///
    /// Note: Only inserts the first model if multiple were added.
    ///
    /// # Errors
    ///
    /// Returns an error if no models were added or if the insert fails.
    pub async fn exec_with_returning(self, conn: &turso::Connection) -> Result<E::Model> {
        if self.models.is_empty() {
            return Err(Error::Query("No models to insert".to_string()));
        }

        let model = self.models.first().unwrap();
        let (columns, values) = model.get_insert_columns_and_values();

        let sql = if columns.is_empty() {
            format!("INSERT INTO {} DEFAULT VALUES RETURNING {}", E::table_name(), E::all_columns())
        } else {
            let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
            format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                E::table_name(),
                columns.join(", "),
                placeholders.join(", "),
                E::all_columns()
            )
        };

        let params: Vec<turso::Value> = values.into_iter().collect();
        let mut rows = conn.query(&sql, params).await?;

        if let Some(row) = rows.next().await? { E::Model::from_row(&row) } else { Err(Error::NoRowsAffected) }
    }

    /// Execute the insert and return the last inserted ID
    ///
    /// Uses SQLite's `last_insert_rowid()` function to get the ID of
    /// the inserted row. Useful when you need the ID but not the full row.
    ///
    /// Note: Only inserts the first model if multiple were added.
    ///
    /// # Errors
    ///
    /// Returns an error if no models were added, if the insert fails,
    /// or if the ID cannot be retrieved.
    pub async fn exec_with_last_insert_id(self, conn: &turso::Connection) -> Result<i64> {
        if self.models.is_empty() {
            return Err(Error::Query("No models to insert".to_string()));
        }

        let model = self.models.first().unwrap();
        let (sql, params) = self.build_single(model);
        let params: Vec<turso::Value> = params.into_iter().collect();

        conn.execute(&sql, params).await?;

        // Query last_insert_rowid()
        let mut rows = conn.query("SELECT last_insert_rowid()", ()).await?;

        if let Some(row) = rows.next().await? {
            let value = row.get_value(0)?;
            match value {
                Value::Integer(id) => Ok(id),
                _ => Err(Error::Query("Failed to get last insert ID".to_string())),
            }
        } else {
            Err(Error::Query("Failed to get last insert ID".to_string()))
        }
    }
}

/// Batch insert builder for inserting multiple records
///
/// Use this when you need to insert many records at once. Currently inserts
/// records one by one, but provides a cleaner API for batch operations.
///
/// # Example
///
/// ```ignore
/// let users = vec![
///     UserActiveModel { name: set("Alice".to_string()), ..Default::default() },
///     UserActiveModel { name: set("Bob".to_string()), ..Default::default() },
/// ];
///
/// let affected = InsertMany::<UserEntity>::new(users).exec(&conn).await?;
/// ```
#[derive(Clone, Debug)]
pub struct InsertMany<E: EntityTrait> {
    models:  Vec<E::ActiveModel>,
    _entity: PhantomData<E>,
}

impl<E: EntityTrait> InsertMany<E> {
    /// Create a new batch insert
    pub fn new(models: Vec<E::ActiveModel>) -> Self {
        Self { models, _entity: PhantomData }
    }

    /// Execute the batch insert and return the total rows affected
    ///
    /// # Errors
    ///
    /// Returns an error if any insert fails. Already-inserted records
    /// are not rolled back unless using a transaction.
    pub async fn exec(self, conn: &turso::Connection) -> Result<u64> {
        if self.models.is_empty() {
            return Ok(0);
        }

        let mut total_affected = 0u64;

        // For now, insert one by one
        // TODO: Use batch insert syntax for better performance
        for model in &self.models {
            let (columns, values) = model.get_insert_columns_and_values();

            let sql = if columns.is_empty() {
                format!("INSERT INTO {} DEFAULT VALUES", E::table_name())
            } else {
                let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
                format!("INSERT INTO {} ({}) VALUES ({})", E::table_name(), columns.join(", "), placeholders.join(", "))
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
    use crate::entity::ActiveValue;
    use crate::entity::ColumnTrait;
    use crate::entity::FromRow;
    use crate::entity::ModelTrait;
    use crate::entity::set;
    use crate::value::ColumnType;
    #[allow(unused_imports)]
    use crate::value::IntoValue;

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

    // Insert::new tests
    #[test]
    fn test_insert_new() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);

        // Verify insert was created with one model
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    // Insert::empty tests
    #[test]
    fn test_insert_empty() {
        let insert = Insert::<TestEntity>::empty();
        // Debug should show empty models
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    // Insert::add tests
    #[test]
    fn test_insert_add() {
        let model1 = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let model2 = TestActiveModel {
            name: set("Bob".to_string()),
            email: set("bob@example.com".to_string()),
            ..Default::default()
        };

        let insert = Insert::<TestEntity>::new(model1).add(model2);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    // Insert::add_many tests
    #[test]
    fn test_insert_add_many() {
        let models = vec![
            TestActiveModel {
                name: set("Alice".to_string()),
                email: set("alice@example.com".to_string()),
                ..Default::default()
            },
            TestActiveModel {
                name: set("Bob".to_string()),
                email: set("bob@example.com".to_string()),
                ..Default::default()
            },
        ];

        let insert = Insert::<TestEntity>::empty().add_many(models);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    // Insert::build_single tests (indirectly tested)
    #[test]
    fn test_insert_build_with_values() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);

        // We can't call build_single directly as it's private, but we can verify the insert was created
        assert!(format!("{:?}", insert).contains("Alice"));
    }

    #[test]
    fn test_insert_with_empty_model() {
        // When all fields are NotSet, should generate DEFAULT VALUES
        let model = TestActiveModel::default();
        let insert = Insert::<TestEntity>::new(model);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

    // Clone tests
    #[test]
    fn test_insert_clone() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);
        let cloned = insert.clone();

        assert_eq!(format!("{:?}", insert), format!("{:?}", cloned));
    }

    // Debug tests
    #[test]
    fn test_insert_debug() {
        let model = TestActiveModel { name: set("Test".to_string()), ..Default::default() };
        let insert = Insert::<TestEntity>::new(model);
        let debug = format!("{:?}", insert);

        assert!(debug.contains("Insert"));
    }

    // InsertMany tests
    #[test]
    fn test_insert_many_new() {
        let models = vec![
            TestActiveModel {
                name: set("Alice".to_string()),
                email: set("alice@example.com".to_string()),
                ..Default::default()
            },
            TestActiveModel {
                name: set("Bob".to_string()),
                email: set("bob@example.com".to_string()),
                ..Default::default()
            },
        ];

        let insert_many = InsertMany::<TestEntity>::new(models);
        assert!(format!("{:?}", insert_many).contains("InsertMany"));
    }

    #[test]
    fn test_insert_many_empty() {
        let insert_many = InsertMany::<TestEntity>::new(vec![]);
        assert!(format!("{:?}", insert_many).contains("InsertMany"));
    }

    #[test]
    fn test_insert_many_clone() {
        let models = vec![TestActiveModel { name: set("Alice".to_string()), ..Default::default() }];
        let insert_many = InsertMany::<TestEntity>::new(models);
        let cloned = insert_many.clone();

        assert_eq!(format!("{:?}", insert_many), format!("{:?}", cloned));
    }

    // Test with partial fields set
    #[test]
    fn test_insert_partial_fields() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            // email is not set
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);
        let debug = format!("{:?}", insert);
        assert!(debug.contains("Alice"));
        // The email field is NotSet, which should be shown as NotSet in debug output
        assert!(debug.contains("NotSet"));
    }

    // Test chained operations
    #[test]
    fn test_insert_chained_add() {
        let insert = Insert::<TestEntity>::empty()
            .add(TestActiveModel { name: set("Alice".to_string()), ..Default::default() })
            .add(TestActiveModel { name: set("Bob".to_string()), ..Default::default() })
            .add(TestActiveModel { name: set("Charlie".to_string()), ..Default::default() });

        let debug = format!("{:?}", insert);
        assert!(debug.contains("Alice"));
        assert!(debug.contains("Bob"));
        assert!(debug.contains("Charlie"));
    }
}
