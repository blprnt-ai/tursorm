# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Tursorm is a simple ORM (Object-Relational Mapping) library for Turso databases, inspired by SeaORM. It provides type-safe database operations for Rust applications using SQLite/Turso as the backend.

## Building and Testing

### Build the project
```powershell
cargo build
```

### Run all tests
```powershell
cargo test
```

### Run tests for a specific module
```powershell
cargo test --test <test_name>
cargo test --lib <module_path>
```

For example, to test only the migration module:
```powershell
cargo test migration
```

### Format code
```powershell
cargo fmt
```

The project uses custom formatting rules defined in `rustfmt.toml`:
- Max width: 120 characters
- 4 spaces for indentation
- Reorder imports by: Std, External Crates, then local crates
- Import granularity: Item level

### Lint code
```powershell
cargo clippy
```

### Check code without building
```powershell
cargo check
```

## Architecture Overview

### Core Components

**1. Connection Layer** (`src/connection/`)
- `Connection`: Main database connection wrapper around `turso::Connection`
- `Builder`: Fluent API for creating database connections with options
- `DatabaseOpts`: Configuration for database connection (MVCC, encryption, path)

**2. Query Builders** (`src/query/`)
- `Select`: Build SELECT queries with filtering, ordering, limits
- `Insert`: Build INSERT queries for single/multiple records
- `Update`: Build UPDATE queries with conditions
- `Delete`: Build DELETE queries with conditions
- `Condition`: Type-safe query conditions (eq, ne, gt, lt, is_in, like, etc.)

**3. Traits System** (`src/traits/`)
- `TableTrait`: Defines table metadata (name, columns, primary key)
- `RecordTrait`: Represents a database row/record
- `ColumnTrait`: Defines column metadata (name, type, constraints)
- `ChangeSetTrait`: Tracks field changes for inserts/updates
- `FromRow`: Converts database rows to Rust types
- `FieldValue<T>`: Enum for tracking field changes (Set/NotSet)

**4. Type Conversion** (`src/value.rs`)
- `IntoValue`: Convert Rust types to database values
- `FromValue`: Convert database values to Rust types
- Support for primitives, strings, blobs, Option<T>, Vec<T> (with features)
- Optional support for chrono (datetime), uuid, JSON via features

**5. Schema Migration** (`src/migration.rs`)
- `Migrator`: Automatic schema migration by comparing code definitions with database
- `SchemaDiff`: Tracks schema changes (create table, add/drop column, rename, etc.)
- `TableSchema`: Represents table structure from code
- Supports column renames, defaults, foreign keys, unique constraints

**6. Procedural Macros** (`tursorm-macros/`)
- `#[derive(Table)]`: Generates TableTrait, Column enum, Record, and ChangeSet implementations
- Attributes: `#[tursorm(primary_key, auto_increment, unique, column_name, default, renamed_from, foreign_key, references)]`

### Key Design Patterns

**Macro-Generated Code**: The `#[derive(Table)]` macro generates:
- Table struct implementing `TableTrait`
- Column enum implementing `ColumnTrait` with all columns as variants
- Record struct with actual field values
- ChangeSet struct for tracking modifications

**Query Builder Pattern**: All queries use method chaining:
```rust
Table::find()
    .filter(Condition::eq(Column::Name, "value"))
    .order_by_desc(Column::CreatedAt)
    .limit(10)
    .all(&conn)
    .await
```

**Change Tracking**: Updates use `FieldValue<T>` enum to track which fields changed:
- `FieldValue::Set(value)`: Field was modified
- `FieldValue::NotSet`: Field unchanged

**Type-Safe Queries**: Column enums prevent typos and provide compile-time guarantees.

### Cargo Features

- `default`: Enables `with-arrays` and `serde`
- `with-arrays`: JSON array support for Vec<String>, Vec<i64>, etc.
- `with-json`: JSON field support via serde_json
- `with-chrono`: DateTime support (NaiveDateTime, NaiveDate, etc.)
- `with-uuid`: UUID field support
- `serde`: Serialization support

## Code Conventions

### Rust Edition and Toolchain
- Rust edition: 2024
- Toolchain version: 1.92.0 (defined in `rust-toolchain.toml`)
- Required components: rustfmt, clippy

### Module Organization
- `pub(crate)` for internal modules not exposed in public API
- `pub mod prelude` for commonly used exports
- Comprehensive test coverage with `#[cfg(test)] mod tests`

### Error Handling
- Custom `Error` enum using thiserror
- `Result<T>` type alias for `std::result::Result<T, Error>`
- Detailed error messages with context

### Async/Await
- All database operations are async using tokio runtime
- Use `async-trait` crate for async trait methods
- Connection methods return `turso::Result<T>` or custom `Result<T>`

### Testing Patterns
- Unit tests in each module under `#[cfg(test)] mod tests`
- Integration tests use `ctor` for setup
- Mock implementations for testing traits
- Use `fake` crate for test data generation

## Important Notes

### Foreign Keys
Foreign key ON DELETE and ON UPDATE constraints are parsed but not yet fully implemented in SQL generation (see `migration.rs` lines 643-662).

### Transactions
Transactions are currently commented out due to issues with the underlying turso library (see `connection/mod.rs` lines 23-30). Do not attempt to use `conn.begin()`.

### MVCC Mode
When MVCC is disabled, unique constraints are enforced via indexes. When MVCC is enabled, unique constraints can be part of table definition (see `migration.rs` lines 481-500).

### Windows Compatibility
This project supports Windows development (using PowerShell as shown in the environment context).
