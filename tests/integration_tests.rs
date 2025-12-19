//! Integration tests for tursorm using in-memory SQLite database
//!
//! These tests verify the full ORM workflow including:
//! - Table creation and schema management
//! - CRUD operations (Create, Read, Update, Delete)
//! - Query builder functionality
//! - Migrations
//! - Error handling

use tursorm::prelude::*;

// =============================================================================
// Test Entity Definitions
// =============================================================================

/// User entity for testing basic CRUD operations
#[derive(Clone, Debug, PartialEq, Entity)]
#[tursorm(table_name = "users")]
pub struct User {
    #[tursorm(primary_key, auto_increment)]
    pub id:    i64,
    pub name:  String,
    pub email: String,
    pub age:   Option<i64>,
}

/// Post entity for testing relationships and more complex queries
#[derive(Clone, Debug, PartialEq, Entity)]
#[tursorm(table_name = "posts")]
pub struct Post {
    #[tursorm(primary_key, auto_increment)]
    pub id:        i64,
    pub user_id:   i64,
    pub title:     String,
    pub content:   String,
    pub published: i64, // SQLite doesn't have bool, use 0/1
}

/// Product entity for testing default values and unique constraints
#[derive(Clone, Debug, PartialEq, Entity)]
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

// =============================================================================
// Helper Functions
// =============================================================================

/// Create an in-memory database connection for testing
async fn create_test_db() -> Connection {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    db.connect().unwrap()
}

/// Create the users table
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

/// Create the posts table
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

/// Create the products table
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

/// Insert sample users and return them
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
        let model = UserActiveModel {
            name: set(name.to_string()),
            email: set(email.to_string()),
            age: set(age),
            ..Default::default()
        };
        // Use exec and then query back to avoid potential RETURNING issues
        Insert::<UserEntity>::new(model).exec(conn).await.unwrap();

        // Query back the inserted user
        let user = Select::<UserEntity>::new()
            .filter(Condition::eq(UserColumn::Id, (idx + 1) as i64))
            .one(conn)
            .await
            .unwrap()
            .unwrap();
        users.push(user);
    }
    users
}

// =============================================================================
// Schema Tests
// =============================================================================

mod schema_tests {
    use tursorm::MigrationSchema;

    use super::*;

    #[tokio::test]
    async fn test_create_table_sql_generation() {
        let sql = MigrationSchema::create_table_sql::<UserEntity>(false);
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("name TEXT NOT NULL"));
        assert!(sql.contains("email TEXT NOT NULL"));
        assert!(sql.contains("age INTEGER")); // nullable, no NOT NULL
    }

    #[tokio::test]
    async fn test_create_table_if_not_exists() {
        let sql = MigrationSchema::create_table_sql::<UserEntity>(true);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS users"));
    }

    #[tokio::test]
    async fn test_drop_table_sql_generation() {
        let sql = MigrationSchema::drop_table_sql::<UserEntity>(false);
        assert_eq!(sql, "DROP TABLE users");

        let sql_if_exists = MigrationSchema::drop_table_sql::<UserEntity>(true);
        assert_eq!(sql_if_exists, "DROP TABLE IF EXISTS users");
    }

    #[tokio::test]
    async fn test_schema_create_table() {
        let conn = create_test_db().await;

        // Create table using Schema helper
        MigrationSchema::create_table::<UserEntity>(&conn, false).await.unwrap();

        // Verify table exists by inserting a record
        let model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_drop_table() {
        let conn = create_test_db().await;

        MigrationSchema::create_table::<UserEntity>(&conn, false).await.unwrap();
        MigrationSchema::drop_table::<UserEntity>(&conn, false).await.unwrap();

        // Verify table is gone - inserting should fail
        let model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_err());
    }
}

// =============================================================================
// Insert Tests
// =============================================================================

mod insert_tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let model = UserActiveModel {
            name: set("Alice".to_string()),
            email: set("alice@example.com".to_string()),
            age: set(Some(30)),
            ..Default::default()
        };

        let affected = Insert::<UserEntity>::new(model).exec(&conn).await.unwrap();
        assert_eq!(affected, 1);
    }

    #[tokio::test]
    async fn test_insert_with_last_insert_id() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let model = UserActiveModel {
            name: set("Charlie".to_string()),
            email: set("charlie@example.com".to_string()),
            ..Default::default()
        };

        let id = Insert::<UserEntity>::new(model).exec_with_last_insert_id(&conn).await.unwrap();

        assert_eq!(id, 1);

        // Insert another and verify ID increments
        let model2 = UserActiveModel {
            name: set("Diana".to_string()),
            email: set("diana@example.com".to_string()),
            ..Default::default()
        };

        let id2 = Insert::<UserEntity>::new(model2).exec_with_last_insert_id(&conn).await.unwrap();

        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_insert_multiple_records() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let insert = Insert::<UserEntity>::empty()
            .add(UserActiveModel {
                name: set("User1".to_string()),
                email: set("user1@test.com".to_string()),
                ..Default::default()
            })
            .add(UserActiveModel {
                name: set("User2".to_string()),
                email: set("user2@test.com".to_string()),
                ..Default::default()
            })
            .add(UserActiveModel {
                name: set("User3".to_string()),
                email: set("user3@test.com".to_string()),
                ..Default::default()
            });

        let affected = insert.exec(&conn).await.unwrap();
        assert_eq!(affected, 3);

        // Verify all were inserted
        let users = Select::<UserEntity>::new().all(&conn).await.unwrap();
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_insert_many() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let models = vec![
            UserActiveModel {
                name: set("Batch1".to_string()),
                email: set("batch1@test.com".to_string()),
                ..Default::default()
            },
            UserActiveModel {
                name: set("Batch2".to_string()),
                email: set("batch2@test.com".to_string()),
                ..Default::default()
            },
        ];

        let affected = InsertMany::<UserEntity>::new(models).exec(&conn).await.unwrap();
        assert_eq!(affected, 2);
    }
}

// =============================================================================
// Select Tests
// =============================================================================

mod select_tests {
    use super::*;

    #[tokio::test]
    async fn test_select_all() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let inserted_users = insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().all(&conn).await.unwrap();
        assert_eq!(users.len(), inserted_users.len());
    }

    #[tokio::test]
    async fn test_select_one() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Id, 1)).one(&conn).await.unwrap();

        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_select_one_not_found() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Id, 999)).one(&conn).await.unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_select_with_eq_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Name, "Bob")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Bob");
    }

    #[tokio::test]
    async fn test_select_with_ne_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::ne(UserColumn::Name, "Alice")).all(&conn).await.unwrap();

        assert_eq!(users.len(), 4);
        assert!(users.iter().all(|u| u.name != "Alice"));
    }

    #[tokio::test]
    async fn test_select_with_gt_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().filter(Condition::gt(UserColumn::Age, 28)).all(&conn).await.unwrap();

        // Alice (30), Diana (35)
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_gte_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().filter(Condition::gte(UserColumn::Age, 30)).all(&conn).await.unwrap();

        // Alice (30), Diana (35)
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_lt_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().filter(Condition::lt(UserColumn::Age, 28)).all(&conn).await.unwrap();

        // Bob (25)
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Bob");
    }

    #[tokio::test]
    async fn test_select_with_like_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new()
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
            Select::<UserEntity>::new().filter(Condition::contains(UserColumn::Name, "li")).all(&conn).await.unwrap();

        // Alice, Charlie
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_starts_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::starts_with(UserColumn::Name, "A")).all(&conn).await.unwrap();

        // Alice
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_select_with_ends_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::ends_with(UserColumn::Name, "e")).all(&conn).await.unwrap();

        // Alice, Charlie, Eve
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_is_null_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().filter(Condition::is_null(UserColumn::Age)).all(&conn).await.unwrap();

        // Charlie has no age
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_with_is_not_null_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::is_not_null(UserColumn::Age)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 4);
    }

    #[tokio::test]
    async fn test_select_with_in_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new()
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
            Select::<UserEntity>::new().filter(Condition::not_in(UserColumn::Id, vec![1, 2])).all(&conn).await.unwrap();

        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_between_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().filter(Condition::between(UserColumn::Age, 25, 30)).all(&conn).await.unwrap();

        // Bob (25), Eve (28), Alice (30)
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_and_conditions() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new()
            .filter(Condition::gt(UserColumn::Age, 25).and(Condition::lt(UserColumn::Age, 35)))
            .all(&conn)
            .await
            .unwrap();

        // Eve (28), Alice (30)
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_or_conditions() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new()
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

        let users = Select::<UserEntity>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .filter(Condition::gt(UserColumn::Age, 25))
            .all(&conn)
            .await
            .unwrap();

        // Eve (28), Alice (30), Diana (35)
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_select_with_order_by_asc() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .order_by_asc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users[0].name, "Bob"); // 25
        assert_eq!(users[1].name, "Eve"); // 28
        assert_eq!(users[2].name, "Alice"); // 30
        assert_eq!(users[3].name, "Diana"); // 35
    }

    #[tokio::test]
    async fn test_select_with_order_by_desc() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().order_by_desc(UserColumn::Name).all(&conn).await.unwrap();

        assert_eq!(users[0].name, "Eve");
        assert_eq!(users[1].name, "Diana");
    }

    #[tokio::test]
    async fn test_select_with_limit() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = Select::<UserEntity>::new().limit(2).all(&conn).await.unwrap();

        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_select_with_offset() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // SQLite requires LIMIT with OFFSET, so we use a large limit
        let users = Select::<UserEntity>::new()
            .order_by_asc(UserColumn::Id)
            .limit(1000) // Large limit to get all remaining
            .offset(2)
            .all(&conn)
            .await
            .unwrap();

        // Skip Alice and Bob
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_with_limit_and_offset() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users =
            Select::<UserEntity>::new().order_by_asc(UserColumn::Id).limit(2).offset(1).all(&conn).await.unwrap();

        // Skip Alice, get Bob and Charlie
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Bob");
        assert_eq!(users[1].name, "Charlie");
    }

    #[tokio::test]
    async fn test_select_count() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let count = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(count, 5);

        let count_with_filter =
            Select::<UserEntity>::new().filter(Condition::is_not_null(UserColumn::Age)).count(&conn).await.unwrap();
        assert_eq!(count_with_filter, 4);
    }

    #[tokio::test]
    async fn test_select_exists() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let exists =
            Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Name, "Alice")).exists(&conn).await.unwrap();
        assert!(exists);

        let not_exists = Select::<UserEntity>::new()
            .filter(Condition::eq(UserColumn::Name, "NotExist"))
            .exists(&conn)
            .await
            .unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_select_specific_columns() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // This tests the columns() method - note that FromRow still expects all columns
        // in order, so this is mainly useful for raw queries
        let (sql, _) = Select::<UserEntity>::new().columns(vec![UserColumn::Id, UserColumn::Name]).build();

        assert!(sql.contains("SELECT id, name FROM"));
        assert!(!sql.contains("email"));
    }
}

// =============================================================================
// Update Tests
// =============================================================================

mod update_tests {
    use super::*;

    #[tokio::test]
    async fn test_update_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        let mut update_model = UserActiveModel::from(users[0].clone());
        update_model.name = set("Alice Updated".to_string());

        let affected = Update::<UserEntity>::new(update_model).exec(&conn).await.unwrap();
        assert_eq!(affected, 1);

        // Verify update
        let updated_user = Select::<UserEntity>::new()
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

        let affected = Update::<UserEntity>::many()
            .set(UserColumn::Age, 99i64)
            .filter(Condition::gt(UserColumn::Age, 30))
            .exec(&conn)
            .await
            .unwrap();

        // Diana (35) should be updated
        assert_eq!(affected, 1);

        // Verify
        let users = Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Age, 99i64)).all(&conn).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Diana");
    }

    #[tokio::test]
    async fn test_update_multiple_columns() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        let users = insert_sample_users(&conn).await;

        let mut update_model = UserActiveModel::from(users[0].clone());
        update_model.name = set("New Name".to_string());
        update_model.email = set("new@email.com".to_string());
        update_model.age = set(Some(50));

        Update::<UserEntity>::new(update_model).exec(&conn).await.unwrap();

        let updated = Select::<UserEntity>::new()
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

        // Alice has age = 30
        let mut update_model = UserActiveModel::from(users[0].clone());
        update_model.age = set(None);

        Update::<UserEntity>::new(update_model).exec(&conn).await.unwrap();

        let updated = Select::<UserEntity>::new()
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

        // Update with no columns set should error
        let result = Update::<UserEntity>::many().filter(Condition::eq(UserColumn::Id, 1)).exec(&conn).await;

        assert!(result.is_err());
    }
}

// =============================================================================
// Delete Tests
// =============================================================================

mod delete_tests {
    use super::*;

    #[tokio::test]
    async fn test_delete_single_record() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserEntity>::new().filter(Condition::eq(UserColumn::Id, 1)).exec(&conn).await.unwrap();

        assert_eq!(affected, 1);

        // Verify deletion
        let count = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_delete_with_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Delete users with age > 30
        let affected =
            Delete::<UserEntity>::new().filter(Condition::gt(UserColumn::Age, 30)).exec(&conn).await.unwrap();

        // Diana (35)
        assert_eq!(affected, 1);
    }

    #[tokio::test]
    async fn test_delete_with_in_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected = Delete::<UserEntity>::new()
            .filter(Condition::is_in(UserColumn::Id, vec![1, 2, 3]))
            .exec(&conn)
            .await
            .unwrap();

        assert_eq!(affected, 3);

        let remaining = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(remaining, 2);
    }

    #[tokio::test]
    async fn test_delete_with_like_condition() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Delete users whose name ends with 'e'
        let affected =
            Delete::<UserEntity>::new().filter(Condition::like(UserColumn::Name, "%e")).exec(&conn).await.unwrap();

        // Alice, Charlie, Eve
        assert_eq!(affected, 3);
    }

    #[tokio::test]
    async fn test_delete_no_match() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let affected =
            Delete::<UserEntity>::new().filter(Condition::eq(UserColumn::Id, 999)).exec(&conn).await.unwrap();

        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn test_delete_all() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Delete without filter removes all
        let affected = Delete::<UserEntity>::new().exec(&conn).await.unwrap();
        assert_eq!(affected, 5);

        let count = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(count, 0);
    }
}

// =============================================================================
// Model Extension Tests
// =============================================================================

mod model_ext_tests {

    use tursorm::EntitySelectExt;

    use super::*;

    #[tokio::test]
    async fn test_model_find() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = UserEntity::find().all(&conn).await.unwrap();
        assert_eq!(users.len(), 5);
    }

    #[tokio::test]
    async fn test_model_find_by_id() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let user = UserEntity::find_by_id(1).one(&conn).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_model_find_with_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let users = UserEntity::find()
            .filter(Condition::gt(UserColumn::Age, 25))
            .order_by_desc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(users[0].name, "Diana"); // 35
    }
}

// =============================================================================
// Migration Tests
// =============================================================================

mod migration_tests {
    use tursorm::migration::EntitySchema;
    use tursorm::migration::MigrationOptions;
    use tursorm::migration::Migrator;
    use tursorm::migration::SchemaDiff;

    use super::*;

    #[tokio::test]
    async fn test_migrate_creates_new_table() {
        let conn = create_test_db().await;

        // Table doesn't exist yet
        let diff = Migrator::migrate::<UserEntity>(&conn).await.unwrap();

        assert!(diff.has_changes);
        assert!(!diff.has_warnings);

        // Verify table was created by inserting
        let model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_migrate_no_changes_for_existing_table() {
        let conn = create_test_db().await;

        // First migration creates table
        Migrator::migrate::<UserEntity>(&conn).await.unwrap();

        // Second migration should have no changes
        let diff = Migrator::migrate::<UserEntity>(&conn).await.unwrap();

        assert!(!diff.has_changes);
        assert!(!diff.has_warnings);
    }

    #[tokio::test]
    async fn test_migrate_dry_run() {
        let conn = create_test_db().await;

        let options = MigrationOptions { dry_run: true, ..Default::default() };

        let diff = Migrator::migrate_with_options::<UserEntity>(&conn, options).await.unwrap();

        // Should report changes but not apply them
        assert!(diff.has_changes);

        // Table should NOT exist
        let model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        let result = Insert::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_migrate_multiple_entities() {
        let conn = create_test_db().await;

        let schemas = vec![EntitySchema::of::<UserEntity>(), EntitySchema::of::<PostEntity>()];

        let diff = Migrator::migrate_all(&conn, &schemas).await.unwrap();

        assert!(diff.has_changes);

        // Verify both tables were created
        let user_model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };
        assert!(Insert::<UserEntity>::new(user_model).exec(&conn).await.is_ok());

        let post_model = PostActiveModel {
            user_id: set(1),
            title: set("Test Post".to_string()),
            content: set("Content".to_string()),
            published: set(1),
            ..Default::default()
        };
        assert!(Insert::<PostEntity>::new(post_model).exec(&conn).await.is_ok());
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

        // Check column names
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
        let schema = EntitySchema::of::<UserEntity>();

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

// =============================================================================
// Error Handling Tests
// =============================================================================

mod error_tests {
    use tursorm::Error;

    use super::*;

    #[tokio::test]
    async fn test_insert_into_nonexistent_table() {
        let conn = create_test_db().await;
        // Don't create table

        let model = UserActiveModel {
            name: set("Test".to_string()),
            email: set("test@test.com".to_string()),
            ..Default::default()
        };

        let result = Insert::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_select_from_nonexistent_table() {
        let conn = create_test_db().await;

        let result = Select::<UserEntity>::new().all(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_without_primary_key_or_filter() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        // ActiveModel without primary key set and no filter
        let model = UserActiveModel { name: set("Test".to_string()), ..Default::default() };

        let result = Update::<UserEntity>::new(model).exec(&conn).await;
        assert!(result.is_err());

        if let Err(Error::PrimaryKeyNotSet) = result {
            // Expected
        } else {
            panic!("Expected PrimaryKeyNotSet error");
        }
    }

    #[tokio::test]
    async fn test_insert_empty_returns_zero() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;

        let affected = Insert::<UserEntity>::empty().exec(&conn).await.unwrap();
        assert_eq!(affected, 0);
    }
}

// =============================================================================
// Complex Query Tests
// =============================================================================

mod complex_query_tests {
    use super::*;

    #[tokio::test]
    async fn test_pagination() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Page 1 (first 2 users)
        let page1 =
            Select::<UserEntity>::new().order_by_asc(UserColumn::Id).limit(2).offset(0).all(&conn).await.unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].name, "Alice");
        assert_eq!(page1[1].name, "Bob");

        // Page 2 (next 2 users)
        let page2 =
            Select::<UserEntity>::new().order_by_asc(UserColumn::Id).limit(2).offset(2).all(&conn).await.unwrap();

        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].name, "Charlie");
        assert_eq!(page2[1].name, "Diana");

        // Page 3 (last user)
        let page3 =
            Select::<UserEntity>::new().order_by_asc(UserColumn::Id).limit(2).offset(4).all(&conn).await.unwrap();

        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].name, "Eve");
    }

    #[tokio::test]
    async fn test_complex_filter_chain() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Complex query: age not null AND (age >= 28 AND age <= 35) AND name doesn't start with 'D'
        let users = Select::<UserEntity>::new()
            .filter(Condition::is_not_null(UserColumn::Age))
            .filter(Condition::gte(UserColumn::Age, 28))
            .filter(Condition::lte(UserColumn::Age, 35))
            .filter(Condition::not_like(UserColumn::Name, "D%"))
            .order_by_asc(UserColumn::Age)
            .all(&conn)
            .await
            .unwrap();

        // Eve (28), Alice (30) - Diana (35) excluded by NOT LIKE
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

        // Create some posts for Alice
        for i in 1..=3 {
            let post = PostActiveModel {
                user_id: set(users[0].id),
                title: set(format!("Post {}", i)),
                content: set(format!("Content {}", i)),
                published: set(1),
                ..Default::default()
            };
            Insert::<PostEntity>::new(post).exec(&conn).await.unwrap();
        }

        // Create posts for Bob
        for i in 1..=2 {
            let post = PostActiveModel {
                user_id: set(users[1].id),
                title: set(format!("Bob's Post {}", i)),
                content: set(format!("Bob's Content {}", i)),
                published: set(0),
                ..Default::default()
            };
            Insert::<PostEntity>::new(post).exec(&conn).await.unwrap();
        }

        // Query Alice's posts
        let alice_posts = Select::<PostEntity>::new()
            .filter(Condition::eq(PostColumn::UserId, users[0].id))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(alice_posts.len(), 3);

        // Query published posts
        let published_posts =
            Select::<PostEntity>::new().filter(Condition::eq(PostColumn::Published, 1i64)).all(&conn).await.unwrap();

        assert_eq!(published_posts.len(), 3);
    }

    #[tokio::test]
    async fn test_update_then_select() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        // Update all users with age > 30 to set age = 100
        Update::<UserEntity>::many()
            .set(UserColumn::Age, 100i64)
            .filter(Condition::gt(UserColumn::Age, 30))
            .exec(&conn)
            .await
            .unwrap();

        // Verify
        let updated_users =
            Select::<UserEntity>::new().filter(Condition::eq(UserColumn::Age, 100i64)).all(&conn).await.unwrap();

        assert_eq!(updated_users.len(), 1);
        assert_eq!(updated_users[0].name, "Diana");
    }

    #[tokio::test]
    async fn test_delete_then_count() {
        let conn = create_test_db().await;
        create_users_table(&conn).await;
        insert_sample_users(&conn).await;

        let initial_count = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(initial_count, 5);

        // Delete users with null age
        Delete::<UserEntity>::new().filter(Condition::is_null(UserColumn::Age)).exec(&conn).await.unwrap();

        let after_delete_count = Select::<UserEntity>::new().count(&conn).await.unwrap();
        assert_eq!(after_delete_count, 4);
    }
}

// =============================================================================
// Product Entity Tests (for unique constraints)
// =============================================================================

mod product_tests {
    use super::*;

    #[tokio::test]
    async fn test_product_crud() {
        let conn = create_test_db().await;
        create_products_table(&conn).await;

        // Insert
        let product = ProductActiveModel {
            name: set("Widget".to_string()),
            sku: set("WGT-001".to_string()),
            price: set(19.99),
            quantity: set(100),
            ..Default::default()
        };

        let id = Insert::<ProductEntity>::new(product).exec_with_last_insert_id(&conn).await.unwrap();

        // Query back the inserted product
        let inserted = Select::<ProductEntity>::new()
            .filter(Condition::eq(ProductColumn::Id, id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(inserted.name, "Widget");
        assert_eq!(inserted.sku, "WGT-001");
        assert!((inserted.price - 19.99).abs() < 0.001);
        assert_eq!(inserted.quantity, 100);

        // Update
        let mut update_model = ProductActiveModel::from(inserted.clone());
        update_model.quantity = set(50);

        Update::<ProductEntity>::new(update_model).exec(&conn).await.unwrap();

        // Verify
        let updated = Select::<ProductEntity>::new()
            .filter(Condition::eq(ProductColumn::Id, inserted.id))
            .one(&conn)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.quantity, 50);

        // Delete
        Delete::<ProductEntity>::new().filter(Condition::eq(ProductColumn::Id, inserted.id)).exec(&conn).await.unwrap();

        let deleted = Select::<ProductEntity>::new()
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

        // Insert products with different prices
        let products =
            vec![("Cheap", "CHE-001", 5.99, 10), ("Medium", "MED-001", 29.99, 20), ("Expensive", "EXP-001", 99.99, 5)];

        for (name, sku, price, qty) in products {
            let model = ProductActiveModel {
                name: set(name.to_string()),
                sku: set(sku.to_string()),
                price: set(price),
                quantity: set(qty),
                ..Default::default()
            };
            Insert::<ProductEntity>::new(model).exec(&conn).await.unwrap();
        }

        // Query products with price > 20
        let expensive =
            Select::<ProductEntity>::new().filter(Condition::gt(ProductColumn::Price, 20.0)).all(&conn).await.unwrap();

        assert_eq!(expensive.len(), 2);

        // Query products between prices
        let mid_range = Select::<ProductEntity>::new()
            .filter(Condition::between(ProductColumn::Price, 10.0, 50.0))
            .all(&conn)
            .await
            .unwrap();

        assert_eq!(mid_range.len(), 1);
        assert_eq!(mid_range[0].name, "Medium");
    }
}
