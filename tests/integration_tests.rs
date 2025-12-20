use fake::Fake;
use tursorm::prelude::*;

macro_rules! before_all {
    (init = $path:path) => {
        #[allow(non_snake_case)]
        #[ctor::ctor]
        fn __BEFORE_ALL__() {
            static ONCE: std::sync::Once = std::sync::Once::new();
            ONCE.call_once(|| {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
                rt.block_on(async { $path().await });
            });
        }
    };
}

async fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::new("warn").add_directive("tursorm=trace".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();
}

before_all!(init = init_tracing);

#[derive(Clone, Debug, PartialEq, Table)]
#[tursorm(table_name = "users")]
pub struct User {
    #[tursorm(primary_key, auto_increment)]
    pub id:    i64,
    pub name:  String,
    pub email: String,
    pub age:   Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Table)]
#[tursorm(table_name = "posts")]
pub struct Post {
    #[tursorm(primary_key, auto_increment)]
    pub id:        i64,
    pub user_id:   i64,
    pub title:     String,
    pub content:   String,
    pub published: i64,
}

#[derive(Clone, Debug, PartialEq, Table)]
#[tursorm(table_name = "products")]
pub struct Product {
    #[tursorm(primary_key, auto_increment)]
    pub id:       i64,
    pub name:     String,
    #[tursorm(unique)]
    pub sku:      String,
    pub price:    f64,
    pub quantity: i64,
}

async fn create_test_db() -> Connection {
    let dir = fake::faker::name::en::FirstName().fake::<String>();
    let sub_dir = fake::faker::name::en::FirstName().fake::<String>();
    let tmp_dir = std::env::temp_dir().join("tursorm-test").join(dir).join(sub_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let db_name = fake::faker::name::en::LastName().fake::<String>();

    let mut db_path = tmp_dir.join(db_name);
    db_path.set_extension("db");

    tracing::info!("Creating test database in: {}", db_path.to_string_lossy());

    let db = Builder::new_local(db_path.to_string_lossy().as_ref()).build().await.unwrap();
    db.connect().unwrap()
}

async fn create_users_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            age INTEGER
        )",
        (),
    )
    .await
    .unwrap();
}

async fn create_posts_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            published INTEGER NOT NULL DEFAULT 0
        )",
        (),
    )
    .await
    .unwrap();
}

async fn create_products_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            sku TEXT NOT NULL UNIQUE,
            price REAL NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0
        )",
        (),
    )
    .await
    .unwrap();
}

async fn insert_sample_users(conn: &Connection) -> Vec<User> {
    let users_data = vec![
        ("Alice", "alice@example.com", Some(30i64)),
        ("Bob", "bob@example.com", Some(25)),
        ("Charlie", "charlie@example.com", None),
        ("Diana", "diana@example.com", Some(35)),
        ("Eve", "eve@example.com", Some(28)),
    ];

    let mut users = Vec::new();
    for (idx, (name, email, age)) in users_data.into_iter().enumerate() {
        let change_set = UserChangeSet {
            name: set(name.to_string()),
            email: set(email.to_string()),
            age: set(age),
            ..Default::default()
        };

        Insert::<UserTable>::new(change_set).exec(conn).await.unwrap();

        let user = Select::<UserTable>::new()
            .filter(Condition::eq(UserColumn::Id, (idx + 1) as i64))
            .one(conn)
            .await
            .unwrap()
            .unwrap();
        users.push(user);
    }
    users
}

mod schema_tests {
    use tursorm::MigrationSchema;

    use super::*;

    #[tokio::test]
    async fn test_create_table_sql_generation() {
        let sql = MigrationSchema::create_table_sql::<UserTable>(false);
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("name TEXT NOT NULL"));
        assert!(sql.contains("email TEXT NOT NULL"));
        assert!(sql.contains("age INTEGER"));
    }

    #[tokio::test]
    async fn test_create_table_if_not_exists() {
        let sql = MigrationSchema::create_table_sql::<UserTable>(true);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS users"));
    }

    #[tokio::test]
    async fn test_drop_table_sql_generation() {
        let sql = MigrationSchema::drop_table_sql::<UserTable>(false);
        assert_eq!(sql, "DROP TABLE users");

        let sql_if_exists = MigrationSchema::drop_table_sql::<UserTable>(true);
        assert_eq!(sql_if_exists, "DROP TABLE IF EXISTS users");
    }

    #[tokio::test]
    async fn test_schema_create_table() {
        let conn = create_test_db().await;

        MigrationSchema::create_table::<UserTable>(&conn, false).await.unwrap();

        let change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_drop_table() {
        let conn = create_test_db().await;

        MigrationSchema::create_table::<UserTable>(&conn, false).await.unwrap();
        MigrationSchema::drop_table::<UserTable>(&conn, false).await.unwrap();

        let change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_err());
    }
}

mod insert_tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let name = "Alice";
        let email = "alice@example.com";
        let age = Some(30);

        let mut user = UserChangeSet::default();
        user.name = FieldValue::Set(name.to_string());
        user.email = FieldValue::Set(email.to_string());
        user.age = FieldValue::Set(age);

        let user = user.insert(&conn).await.unwrap();

        assert_eq!(user.name, name);
        assert_eq!(user.email, email);
        assert_eq!(user.age, age);
    }

    #[tokio::test]
    async fn test_insert_with_last_insert_id() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let change_set = UserChangeSet {
            name: set("Charlie".to_string()),
            email: set("charlie@example.com".to_string()),
            ..Default::default()
        };

        let id = Insert::<UserTable>::new(change_set).exec_with_last_insert_id(&conn).await.unwrap();

        assert_eq!(id, 1);

        let change_set2 = UserChangeSet {
            name: set("Diana".to_string()),
            email: set("diana@example.com".to_string()),
            ..Default::default()
        };

        let id2 = Insert::<UserTable>::new(change_set2).exec_with_last_insert_id(&conn).await.unwrap();

        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_insert_multiple_records() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let insert = Insert::<UserTable>::empty()
            .add(UserChangeSet {
                name: set("User1".to_string()),
                email: set("user1@test.com".to_string()),
                ..Default::default()
            })
            .add(UserChangeSet {
                name: set("User2".to_string()),
                email: set("user2@test.com".to_string()),
                ..Default::default()
            })
            .add(UserChangeSet {
                name: set("User3".to_string()),
                email: set("user3@test.com".to_string()),
                ..Default::default()
            });

        let affected = insert.exec(&conn).await.unwrap();
        assert_eq!(affected, 3);

        let users = Select::<UserTable>::new().all(&conn).await.unwrap();
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_insert_many() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let change_sets = vec![
            UserChangeSet {
                name: set("Batch1".to_string()),
                email: set("batch1@test.com".to_string()),
                ..Default::default()
            },
            UserChangeSet {
                name: set("Batch2".to_string()),
                email: set("batch2@test.com".to_string()),
                ..Default::default()
            },
        ];

        let affected = InsertMany::<UserTable>::new(change_sets).exec(&conn).await.unwrap();
        assert_eq!(affected, 2);
    }
}

mod select_tests {
    use super::*;

    #[tokio::test]
    async fn test_select_all() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let inserted_users = insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().all(&conn).await.unwrap();
        assert_eq!(users.len(), inserted_users.len());
    }

    #[tokio::test]
    async fn test_select_one() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = Select::<UserTable>::new().filter(Condition::eq(UserColumn::Id, 1)).one(&conn).await.unwrap();

        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_select_one_not_found() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = Select::<UserTable>::new().filter(Condition::eq(UserColumn::Id, 999)).one(&conn).await.unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_select_with_eq_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().filter(Condition::eq(UserColumn::Name, "Bob")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Bob");
    }

    #[tokio::test]
    async fn test_select_with_ne_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::ne(UserColumn::Name, "Alice")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 4);
        assert!(users.iter().all(|u| u.name != "Alice"));
    }

    #[tokio::test]
    async fn test_select_with_gt_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().filter(Condition::gt(UserColumn::Age, 28)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_gte_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().filter(Condition::gte(UserColumn::Age, 30)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_lt_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().filter(Condition::lt(UserColumn::Age, 28)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Bob");
    }

    #[tokio::test]
    async fn test_select_with_like_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::like(UserColumn::Email, "%@example.com"))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 5);
    }

    #[tokio::test]
    async fn test_select_with_contains_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::contains(UserColumn::Name, "li")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_starts_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::starts_with(UserColumn::Name, "A")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_select_with_ends_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::ends_with(UserColumn::Name, "e")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_is_null_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().filter(Condition::is_null(UserColumn::Age)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_with_is_not_null_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::is_not_null(UserColumn::Age)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 4);
    }

    #[tokio::test]
    async fn test_select_with_in_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::is_in(UserColumn::Id, vec![1, 3, 5]))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_not_in_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::not_in(UserColumn::Id, vec![1, 2])).all(&conn).await.unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_between_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().filter(Condition::between(UserColumn::Age, 25, 30)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_and_conditions() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::gt(UserColumn::Age, 25).and(Condition::lt(UserColumn::Age, 35)))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_or_conditions() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::eq(UserColumn::Name, "Alice").or(Condition::eq(UserColumn::Name, "Bob")))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_multiple_filters() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .filter(Condition::gt(UserColumn::Age, 25))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_order_by_asc() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .order_by_asc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users[0].name, "Bob");
        assert_eq!(users[1].name, "Eve");
        assert_eq!(users[2].name, "Alice");
        assert_eq!(users[3].name, "Diana");
    }

    #[tokio::test]
    async fn test_select_with_order_by_desc() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().order_by_desc(UserColumn::Name).all(&conn).await.unwrap();

        assert_eq!(users[0].name, "Eve");
        assert_eq!(users[1].name, "Diana");
    }

    #[tokio::test]
    async fn test_select_with_limit() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new().limit(2).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_offset() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().order_by_asc(UserColumn::Id).limit(1000).offset(2).all(&conn).await.unwrap();

        assert_eq!(users.len(), 3);
        assert_eq!(users[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_with_limit_and_offset() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserTable>::new().order_by_asc(UserColumn::Id).limit(2).offset(1).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Bob");
        assert_eq!(users[1].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_count() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let count = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(count, 5);

        let count_with_filter =
            Select::<UserTable>::new().filter(Condition::is_not_null(UserColumn::Age)).count(&conn).await.unwrap();
        assert_eq!(count_with_filter, 4);
    }

    #[tokio::test]
    async fn test_select_exists() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let exists =
            Select::<UserTable>::new().filter(Condition::eq(UserColumn::Name, "Alice")).exists(&conn).await.unwrap();
        assert!(exists);

        let not_exists =
            Select::<UserTable>::new().filter(Condition::eq(UserColumn::Name, "NotExist")).exists(&conn).await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_select_specific_columns() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let (sql, _) = Select::<UserTable>::new().columns(vec![UserColumn::Id, UserColumn::Name]).build();

        assert!(sql.contains("SELECT id, name FROM"));
        assert!(!sql.contains("email"));
    }
}

mod update_tests {
    use super::*;

    #[tokio::test]
    async fn test_update_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        let mut change_set = UserChangeSet::from(users[0].clone());
        change_set.name = set("Alice Updated".to_string());

        let affected = Update::<UserTable>::new(change_set).exec(&conn).await.unwrap();
        assert_eq!(affected, 1);

        let updated_user = Select::<UserTable>::new()
            .filter(Condition::eq(UserColumn::Id, users[0].id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated_user.name, "Alice Updated");
    }

    #[tokio::test]
    async fn test_update_many_with_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Update::<UserTable>::many()
            .set(UserColumn::Age, 99i64)
            .filter(Condition::gt(UserColumn::Age, 30))
            .exec(&conn)
            .await
            .unwrap();

        assert_eq!(affected, 1);

        let users = Select::<UserTable>::new().filter(Condition::eq(UserColumn::Age, 99i64)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Diana");
    }

    #[tokio::test]
    async fn test_update_multiple_columns() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        let mut change_set = UserChangeSet::from(users[0].clone());
        change_set.name = set("New Name".to_string());
        change_set.email = set("new@email.com".to_string());
        change_set.age = set(Some(50));

        Update::<UserTable>::new(change_set).exec(&conn).await.unwrap();

        let updated = Select::<UserTable>::new()
            .filter(Condition::eq(UserColumn::Id, users[0].id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.email, "new@email.com");
        assert_eq!(updated.age, Some(50));
    }

    #[tokio::test]
    async fn test_update_set_null() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        let mut change_set = UserChangeSet::from(users[0].clone());
        change_set.age = set(None);

        Update::<UserTable>::new(change_set).exec(&conn).await.unwrap();

        let updated = Select::<UserTable>::new()
            .filter(Condition::eq(UserColumn::Id, users[0].id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert!(updated.age.is_none());
    }

    #[tokio::test]
    async fn test_update_no_changes_error() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let result = Update::<UserTable>::many().filter(Condition::eq(UserColumn::Id, 1)).exec(&conn).await;

        assert!(result.is_err());
    }
}

mod delete_tests {
    use super::*;

    #[tokio::test]
    async fn test_delete_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserTable>::new().filter(Condition::eq(UserColumn::Id, 1)).exec(&conn).await.unwrap();

        assert_eq!(affected, 1);

        let count = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_delete_with_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserTable>::new().filter(Condition::gt(UserColumn::Age, 30)).exec(&conn).await.unwrap();

        assert_eq!(affected, 1);
    }

    #[tokio::test]
    async fn test_delete_with_in_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserTable>::new()
            .filter(Condition::is_in(UserColumn::Id, vec![1, 2, 3]))
            .exec(&conn)
            .await
            .unwrap();

        assert_eq!(affected, 3);

        let remaining = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(remaining, 2);
    }

    #[tokio::test]
    async fn test_delete_with_like_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected =
            Delete::<UserTable>::new().filter(Condition::like(UserColumn::Name, "%e")).exec(&conn).await.unwrap();

        assert_eq!(affected, 3);
    }

    #[tokio::test]
    async fn test_delete_no_match() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserTable>::new().filter(Condition::eq(UserColumn::Id, 999)).exec(&conn).await.unwrap();

        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn test_delete_all() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserTable>::new().exec(&conn).await.unwrap();
        assert_eq!(affected, 5);

        let count = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(count, 0);
    }
}

mod change_set_ext_tests {

    use tursorm::TableSelectExt;

    use super::*;

    #[tokio::test]
    async fn test_change_set_find() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = UserTable::find().all(&conn).await.unwrap();
        assert_eq!(users.len(), 5);
    }

    #[tokio::test]
    async fn test_change_set_find_by_id() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = UserTable::find_by_id(1).one(&conn).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_change_set_find_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = UserTable::find()
            .filter(Condition::gt(UserColumn::Age, 25))
            .order_by_desc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users[0].name, "Diana");
    }
}

mod migration_tests {
    use tursorm::migration::MigrationOptions;
    use tursorm::migration::Migrator;
    use tursorm::migration::SchemaDiff;
    use tursorm::migration::TableSchema;

    use super::*;

    #[tokio::test]
    async fn test_migrate_creates_new_table() {
        let conn = create_test_db().await;

        let diff = Migrator::migrate::<UserTable>(&conn).await.unwrap();

        assert!(diff.has_changes);
        assert!(!diff.has_warnings);

        let change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_migrate_no_changes_for_existing_table() {
        let conn = create_test_db().await;

        Migrator::migrate::<UserTable>(&conn).await.unwrap();

        let diff = Migrator::migrate::<UserTable>(&conn).await.unwrap();

        assert!(!diff.has_changes);
        assert!(!diff.has_warnings);
    }

    #[tokio::test]
    async fn test_migrate_dry_run() {
        let conn = create_test_db().await;

        let options = MigrationOptions { dry_run: true, ..Default::default() };

        let diff = Migrator::migrate_with_options::<UserTable>(&conn, options).await.unwrap();

        assert!(diff.has_changes);

        let change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_migrate_multiple_entities() {
        let conn = create_test_db().await;

        let schemas = vec![TableSchema::of::<UserTable>(), TableSchema::of::<PostTable>()];

        let diff = Migrator::migrate_all(&conn, &schemas).await.unwrap();

        assert!(diff.has_changes);

        let user_change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        assert!(Insert::<UserTable>::new(user_change_set).exec(&conn).await.is_ok());

        let post_change_set = PostChangeSet {
            user_id: set(1),
            title: set("Test Post".to_string()),
            content: set("Content".to_string()),
            published: set(1),
            ..Default::default()
        };
        assert!(Insert::<PostTable>::new(post_change_set).exec(&conn).await.is_ok());
    }

    #[tokio::test]
    async fn test_introspect_table() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let table_info = Migrator::introspect_table(&conn, "users").await.unwrap();

        assert!(table_info.is_some());
        let info = table_info.unwrap();
        assert_eq!(info.name, "users");
        assert_eq!(info.columns.len(), 4);

        let col_names: Vec<_> = info.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"id"));
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"email"));
        assert!(col_names.contains(&"age"));
    }

    #[tokio::test]
    async fn test_introspect_nonexistent_table() {
        let conn = create_test_db().await;

        let result = Migrator::introspect_table(&conn, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_entity_schema() {
        let schema = TableSchema::of::<UserTable>();

        assert_eq!(schema.table_name(), "users");
        assert_eq!(schema.columns().len(), 4);

        let id_col = schema.columns().iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.is_primary_key);
        assert!(id_col.is_auto_increment);

        let age_col = schema.columns().iter().find(|c| c.name == "age").unwrap();
        assert!(age_col.nullable);
    }

    #[tokio::test]
    async fn test_schema_diff_summary() {
        let diff = SchemaDiff::empty();
        assert_eq!(diff.summary(), "No changes needed");
    }
}

mod error_tests {
    use tursorm::Error;

    use super::*;

    #[tokio::test]
    async fn test_insert_into_nonexistent_table() {
        let conn = create_test_db().await;

        let change_set = UserChangeSet {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };

        let result = Insert::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_select_from_nonexistent_table() {
        let conn = create_test_db().await;

        let result = Select::<UserTable>::new().all(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_without_primary_key_or_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let change_set = UserChangeSet { name: set("Test".to_string()), ..Default::default() };

        let result = Update::<UserTable>::new(change_set).exec(&conn).await;
        assert!(result.is_err());

        if let Err(Error::PrimaryKeyNotSet) = result {
        } else {
            panic!("Expected PrimaryKeyNotSet error");
        }
    }

    #[tokio::test]
    async fn test_insert_empty_returns_zero() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let affected = Insert::<UserTable>::empty().exec(&conn).await.unwrap();
        assert_eq!(affected, 0);
    }
}

mod complex_query_tests {
    use super::*;

    #[tokio::test]
    async fn test_pagination() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let page1 =
            Select::<UserTable>::new().order_by_asc(UserColumn::Id).limit(2).offset(0).all(&conn).await.unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].name, "Alice");
        assert_eq!(page1[1].name, "Bob");

        let page2 =
            Select::<UserTable>::new().order_by_asc(UserColumn::Id).limit(2).offset(2).all(&conn).await.unwrap();

        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].name, "Charlie");
        assert_eq!(page2[1].name, "Diana");

        let page3 =
            Select::<UserTable>::new().order_by_asc(UserColumn::Id).limit(2).offset(4).all(&conn).await.unwrap();

        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].name, "Eve");
    }

    #[tokio::test]
    async fn test_complex_filter_chain() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserTable>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .filter(Condition::gte(UserColumn::Age, 28))
            .filter(Condition::lte(UserColumn::Age, 35))
            .filter(Condition::not_like(UserColumn::Name, "D%"))
            .order_by_asc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Eve");
        assert_eq!(users[1].name, "Alice");
    }

    #[tokio::test]
    async fn test_posts_by_user() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        create_posts_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        for i in 1..=3 {
            let post = PostChangeSet {
                user_id: set(users[0].id),
                title: set(format!("Post {}", i)),
                content: set(format!("Content {}", i)),
                published: set(1),
                ..Default::default()
            };
            Insert::<PostTable>::new(post).exec(&conn).await.unwrap();
        }

        for i in 1..=2 {
            let post = PostChangeSet {
                user_id: set(users[1].id),
                title: set(format!("Bob's Post {}", i)),
                content: set(format!("Bob's Content {}", i)),
                published: set(0),
                ..Default::default()
            };
            Insert::<PostTable>::new(post).exec(&conn).await.unwrap();
        }

        let alice_posts =
            Select::<PostTable>::new().filter(Condition::eq(PostColumn::UserId, users[0].id)).all(&conn).await.unwrap();

        assert_eq!(alice_posts.len(), 3);

        let published_posts =
            Select::<PostTable>::new().filter(Condition::eq(PostColumn::Published, 1i64)).all(&conn).await.unwrap();

        assert_eq!(published_posts.len(), 3);
    }

    #[tokio::test]
    async fn test_update_then_select() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        Update::<UserTable>::many()
            .set(UserColumn::Age, 100i64)
            .filter(Condition::gt(UserColumn::Age, 30))
            .exec(&conn)
            .await
            .unwrap();

        let updated_users =
            Select::<UserTable>::new().filter(Condition::eq(UserColumn::Age, 100i64)).all(&conn).await.unwrap();

        assert_eq!(updated_users.len(), 1);
        assert_eq!(updated_users[0].name, "Diana");
    }

    #[tokio::test]
    async fn test_delete_then_count() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let initial_count = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(initial_count, 5);

        Delete::<UserTable>::new().filter(Condition::is_null(UserColumn::Age)).exec(&conn).await.unwrap();

        let after_delete_count = Select::<UserTable>::new().count(&conn).await.unwrap();
        assert_eq!(after_delete_count, 4);
    }
}

mod product_tests {
    use super::*;

    #[tokio::test]
    async fn test_product_crud() {
        let conn = create_test_db().await;
        create_products_table(&conn).await;

        let product = ProductChangeSet {
            name: set("Widget".to_string()),
            sku: set("WGT-001".to_string()),
            price: set(19.99),
            quantity: set(100),
            ..Default::default()
        };

        let id = Insert::<ProductTable>::new(product).exec_with_last_insert_id(&conn).await.unwrap();

        let inserted = Select::<ProductTable>::new()
            .filter(Condition::eq(ProductColumn::Id, id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(inserted.name, "Widget");
        assert_eq!(inserted.sku, "WGT-001");
        assert!((inserted.price - 19.99).abs() < 0.001);
        assert_eq!(inserted.quantity, 100);

        let mut change_set = ProductChangeSet::from(inserted.clone());
        change_set.quantity = set(50);

        Update::<ProductTable>::new(change_set).exec(&conn).await.unwrap();

        let updated = Select::<ProductTable>::new()
            .filter(Condition::eq(ProductColumn::Id, inserted.id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.quantity, 50);

        Delete::<ProductTable>::new().filter(Condition::eq(ProductColumn::Id, inserted.id)).exec(&conn).await.unwrap();

        let deleted = Select::<ProductTable>::new()
            .filter(Condition::eq(ProductColumn::Id, inserted.id))
            .one(&conn)
            .await
            .unwrap();

        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_product_float_queries() {
        let conn = create_test_db().await;
        create_products_table(&conn).await;

        let products =
            vec![("Cheap", "CHE-001", 5.99, 10), ("Medium", "MED-001", 29.99, 20), ("Expensive", "EXP-001", 99.99, 5)];

        for (name, sku, price, qty) in products {
            let change_set = ProductChangeSet {
                name: set(name.to_string()),
                sku: set(sku.to_string()),
                price: set(price),
                quantity: set(qty),
                ..Default::default()
            };
            Insert::<ProductTable>::new(change_set).exec(&conn).await.unwrap();
        }

        let expensive =
            Select::<ProductTable>::new().filter(Condition::gt(ProductColumn::Price, 20.0)).all(&conn).await.unwrap();

        assert_eq!(expensive.len(), 2);

        let mid_range = Select::<ProductTable>::new()
            .filter(Condition::between(ProductColumn::Price, 10.0, 50.0))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(mid_range.len(), 1);
        assert_eq!(mid_range[0].name, "Medium");
    }
}
