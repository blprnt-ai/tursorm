# tursorm

A simple ORM for [Turso](https://turso.tech) inspired by SeaORM.

## Installation

```toml
[dependencies]
tursorm = "0.0.1"
```

Optional features: `with-chrono`, `with-uuid`, `with-json`

## Quick Start

```rust
use tursorm::prelude::*;

#[derive(Clone, Debug, Entity)]
#[tursorm(table_name = "users")]
pub struct User {
    #[tursorm(primary_key, auto_increment)]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Builder::new_local(":memory:").build().await?;
    let conn = db.connect()?;

    Schema::create_table::<UserEntity>(&conn, true).await?;

    // Insert
    let user = User::insert(UserActiveModel {
        name: set("Alice".to_string()),
        email: set("alice@example.com".to_string()),
        ..Default::default()
    }).exec_with_returning(&conn).await?;

    // Query
    let users = User::find()
        .filter(Condition::eq(UserColumn::Name, "Alice"))
        .all(&conn).await?;

    // Update
    let mut update: UserActiveModel = user.clone().into();
    update.name = set("Alice Smith".to_string());
    User::update(update).exec(&conn).await?;

    // Delete
    User::delete_by_id(user.id).exec(&conn).await?;

    Ok(())
}
```

## Entity Definition

```rust
#[derive(Clone, Debug, Entity)]
#[tursorm(table_name = "products")]
pub struct Product {
    #[tursorm(primary_key, auto_increment)]
    pub id: i64,
    pub name: String,
    #[tursorm(unique)]
    pub sku: String,
    #[tursorm(column_name = "unit_price")]
    pub price: f64,
    pub description: Option<String>,  // nullable
}
```

**Attributes:**

* `table_name` (struct) - Custom table name
* `primary_key` (field) - Mark as primary key
* `auto_increment` (field) - Auto-incrementing key
* `column_name` (field) - Custom column name
* `unique` (field) - Unique constraint

The macro generates `ProductEntity`, `ProductColumn`, and `ProductActiveModel`.

## Queries

### Select

```rust
let users = User::find().all(&conn).await?;
let user = User::find_by_id(1).one(&conn).await?;

let users = User::find()
    .filter(Condition::eq(UserColumn::Name, "Alice"))
    .filter(Condition::gt(UserColumn::Age, 18))
    .order_by_desc(UserColumn::CreatedAt)
    .limit(10)
    .offset(0)
    .all(&conn).await?;

let count = User::find().count(&conn).await?;
let exists = User::find().filter(...).exists(&conn).await?;
```

### Insert

```rust
let affected = User::insert(model).exec(&conn).await?;
let user = User::insert(model).exec_with_returning(&conn).await?;
let id = User::insert(model).exec_with_last_insert_id(&conn).await?;

InsertMany::<UserEntity>::new(vec![model1, model2]).exec(&conn).await?;
```

### Update

```rust
let mut update: UserActiveModel = user.into();
update.name = set("New Name".to_string());
User::update(update).exec(&conn).await?;

Update::<UserEntity>::many()
    .set(UserColumn::Status, "inactive")
    .filter(Condition::lt(UserColumn::LastLogin, cutoff))
    .exec(&conn).await?;
```

### Delete

```rust
User::delete_by_id(1).exec(&conn).await?;

Delete::<UserEntity>::new()
    .filter(Condition::eq(UserColumn::Status, "deleted"))
    .exec(&conn).await?;
```

## Conditions

```rust
Condition::eq(col, value)           // =
Condition::ne(col, value)           // !=
Condition::gt(col, value)           // >
Condition::gte(col, value)          // >=
Condition::lt(col, value)           // <
Condition::lte(col, value)          // <=
Condition::like(col, pattern)       // LIKE
Condition::contains(col, value)     // LIKE %value%
Condition::starts_with(col, value)  // LIKE value%
Condition::ends_with(col, value)    // LIKE %value
Condition::is_null(col)             // IS NULL
Condition::is_not_null(col)         // IS NOT NULL
Condition::is_in(col, vec)          // IN (...)
Condition::between(col, low, high)  // BETWEEN

// Combine
cond1.and(cond2)
cond1.or(cond2)
cond.not()
```

## Schema & Migrations

```rust
Schema::create_table::<UserEntity>(&conn, true).await?;
Schema::drop_table::<UserEntity>(&conn, true).await?;
Schema::table_exists::<UserEntity>(&conn).await?;

// Auto-migration
use tursorm::migration::{Migrator, MigrationOptions};

Migrator::migrate::<UserEntity>(&conn).await?;

Migrator::migrate_with_options::<UserEntity>(&conn, MigrationOptions {
    allow_drop_columns: false,
    dry_run: true,
    verbose: true,
    ..Default::default()
}).await?;
```

## Connection

```rust
// Local file
let db = Builder::new_local("./data.db").build().await?;

// In-memory
let db = Builder::new_local(":memory:").build().await?;

// Remote Turso
let db = Builder::new_remote(url, token).build().await?;

let conn = db.connect()?;
```

## Supported Types

| Rust | SQLite |
|------|--------|
| `i8`-`i64`, `u8`-`u32` | INTEGER |
| `f32`, `f64` | REAL |
| `String`, `&str` | TEXT |
| `Vec<u8>` | BLOB |
| `bool` | INTEGER (0/1) |
| `Option<T>` | NULL when None |

## License

MIT
