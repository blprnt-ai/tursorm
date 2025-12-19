use std::marker::PhantomData;

use crate::ActiveModelTrait;
use crate::EntityTrait;
use crate::Error;
use crate::Result;
use crate::Value;

#[derive(Clone, Debug)]
pub struct Insert<E: EntityTrait> {
    models:  Vec<E::ActiveModel>,
    _entity: PhantomData<E>,
}

impl<E: EntityTrait> Insert<E> {
    pub fn new(model: E::ActiveModel) -> Self {
        Self { models: vec![model], _entity: PhantomData }
    }

    pub fn empty() -> Self {
        Self { models: Vec::new(), _entity: PhantomData }
    }

    pub fn add(mut self, model: E::ActiveModel) -> Self {
        self.models.push(model);
        self
    }

    pub fn add_many(mut self, models: impl IntoIterator<Item = E::ActiveModel>) -> Self {
        self.models.extend(models);
        self
    }

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

    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
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

    pub async fn exec_with_last_insert_id(self, conn: &crate::Connection) -> Result<i64> {
        if self.models.is_empty() {
            return Err(Error::Query("No models to insert".to_string()));
        }

        let model = self.models.first().unwrap();
        let (sql, params) = self.build_single(model);
        let params: Vec<turso::Value> = params.into_iter().collect();

        conn.execute(&sql, params).await?;
        Ok(conn.last_insert_rowid())
    }
}

#[derive(Clone, Debug)]
pub struct InsertMany<E: EntityTrait> {
    models:  Vec<E::ActiveModel>,
    _entity: PhantomData<E>,
}

impl<E: EntityTrait> InsertMany<E> {
    pub fn new(models: Vec<E::ActiveModel>) -> Self {
        Self { models, _entity: PhantomData }
    }

    pub async fn exec(self, conn: &crate::Connection) -> Result<u64> {
        if self.models.is_empty() {
            return Ok(0);
        }

        let mut total_affected = 0u64;

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
    use crate::ActiveValue;
    use crate::ColumnTrait;
    use crate::ColumnType;
    use crate::FromRow;
    #[allow(unused_imports)]
    use crate::IntoValue;
    use crate::ModelTrait;
    use crate::set;

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

    #[test]
    fn test_insert_new() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);

        assert!(format!("{:?}", insert).contains("Insert"));
    }

    #[test]
    fn test_insert_empty() {
        let insert = Insert::<TestEntity>::empty();

        assert!(format!("{:?}", insert).contains("Insert"));
    }

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

    #[test]
    fn test_insert_build_with_values() {
        let model = TestActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            ..Default::default()
        };
        let insert = Insert::<TestEntity>::new(model);

        assert!(format!("{:?}", insert).contains("Alice"));
    }

    #[test]
    fn test_insert_with_empty_model() {
        let model = TestActiveModel::default();
        let insert = Insert::<TestEntity>::new(model);
        assert!(format!("{:?}", insert).contains("Insert"));
    }

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

    #[test]
    fn test_insert_debug() {
        let model = TestActiveModel { name: set("Test".to_string()), ..Default::default() };
        let insert = Insert::<TestEntity>::new(model);
        let debug = format!("{:?}", insert);

        assert!(debug.contains("Insert"));
    }

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

    #[test]
    fn test_insert_partial_fields() {
        let model = TestActiveModel { name: set("Alice".to_string()), ..Default::default() };
        let insert = Insert::<TestEntity>::new(model);
        let debug = format!("{:?}", insert);
        assert!(debug.contains("Alice"));

        assert!(debug.contains("NotSet"));
    }

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
