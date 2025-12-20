mod schema;

use std::collections::HashMap;

pub use schema::MigrationSchema;

use crate::ForeignKeyInfo;
use crate::OnDelete;
use crate::OnUpdate;
use crate::error::Result;
use crate::traits::column::ColumnTrait;
use crate::traits::table::TableTrait;
use crate::value::ColumnType;

#[derive(Debug, Clone, PartialEq)]
pub struct DbColumnInfo {
    pub name: String,

    pub column_type: String,

    pub nullable: bool,

    pub default_value: Option<String>,

    pub is_primary_key: bool,
}

#[derive(Debug, Clone)]
pub struct DbTableInfo {
    pub name: String,

    pub columns: Vec<DbColumnInfo>,

    pub primary_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum SchemaChange {
    CreateTable { table_name: String, sql: String },

    AddColumn { table_name: String, column_name: String, sql: String },

    DropColumn { table_name: String, column_name: String, sql: String },

    RenameColumn { table_name: String, old_name: String, new_name: String, sql: String },

    RecreateTable { table_name: String, reason: String, sql: Vec<String> },

    CreateIndex { table_name: String, index_name: String, sql: String },

    Warning { table_name: String, message: String },
}

impl SchemaChange {
    pub fn description(&self) -> String {
        match self {
            SchemaChange::CreateTable { table_name, .. } => {
                format!("Create table '{}'", table_name)
            }
            SchemaChange::AddColumn { table_name, column_name, .. } => {
                format!("Add column '{}' to table '{}'", column_name, table_name)
            }
            SchemaChange::DropColumn { table_name, column_name, .. } => {
                format!("Drop column '{}' from table '{}'", column_name, table_name)
            }
            SchemaChange::RenameColumn { table_name, old_name, new_name, .. } => {
                format!("Rename column '{}' to '{}' in table '{}'", old_name, new_name, table_name)
            }
            SchemaChange::RecreateTable { table_name, reason, .. } => {
                format!("Recreate table '{}': {}", table_name, reason)
            }
            SchemaChange::CreateIndex { table_name, index_name, .. } => {
                format!("Create index '{}' on table '{}'", index_name, table_name)
            }
            SchemaChange::Warning { table_name, message } => {
                format!("Warning for '{}': {}", table_name, message)
            }
        }
    }

    pub fn sql_statements(&self) -> Vec<&str> {
        match self {
            SchemaChange::CreateTable { sql, .. } => vec![sql.as_str()],
            SchemaChange::AddColumn { sql, .. } => vec![sql.as_str()],
            SchemaChange::DropColumn { sql, .. } => vec![sql.as_str()],
            SchemaChange::RenameColumn { sql, .. } => vec![sql.as_str()],
            SchemaChange::RecreateTable { sql, .. } => sql.iter().map(|s| s.as_str()).collect(),
            SchemaChange::CreateIndex { sql, .. } => vec![sql.as_str()],
            SchemaChange::Warning { .. } => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaDiff {
    pub changes: Vec<SchemaChange>,

    pub has_changes: bool,

    pub has_warnings: bool,
}

impl SchemaDiff {
    pub fn empty() -> Self {
        Self { changes: Vec::new(), has_changes: false, has_warnings: false }
    }

    pub fn add_change(&mut self, change: SchemaChange) {
        if matches!(change, SchemaChange::Warning { .. }) {
            self.has_warnings = true;
        } else {
            self.has_changes = true;
        }
        self.changes.push(change);
    }

    pub fn all_sql(&self) -> Vec<&str> {
        self.changes.iter().flat_map(|c| c.sql_statements()).collect()
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        for change in &self.changes {
            lines.push(change.description());
        }
        if lines.is_empty() { "No changes needed".to_string() } else { lines.join("\n") }
    }
}

#[derive(Debug, Clone)]
pub enum ForeignKeyChange {
    CreateForeignKey { table_name: String, column_name: String, sql: String },
}

impl ForeignKeyChange {
    pub fn description(&self) -> String {
        match self {
            ForeignKeyChange::CreateForeignKey { table_name, column_name, .. } => {
                format!("Create foreign key '{}' on table '{}'", column_name, table_name)
            }
        }
    }

    pub fn sql_statements(&self) -> Vec<&str> {
        match self {
            ForeignKeyChange::CreateForeignKey { sql, .. } => vec![sql.as_str()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForeignKeyDiff {
    pub changes: Vec<ForeignKeyChange>,

    pub has_changes: bool,
}

impl ForeignKeyDiff {
    pub fn empty() -> Self {
        Self { changes: Vec::new(), has_changes: false }
    }

    pub fn add_change(&mut self, change: ForeignKeyChange) {
        self.has_changes = true;
        self.changes.push(change);
    }

    pub fn expand(&mut self, changes: Vec<ForeignKeyChange>) {
        self.changes.extend(changes);
    }

    pub fn all_sql(&self) -> Vec<&str> {
        self.changes.iter().flat_map(|c| c.sql_statements()).collect()
    }

    pub fn summary(&self) -> String {
        if self.changes.is_empty() {
            "No changes needed".to_string()
        } else {
            self.changes.iter().map(|c| c.description()).collect::<Vec<String>>().join("\n")
        }
    }
}

#[derive(Debug, Clone)]
pub struct MigrationOptions {
    pub allow_drop_columns: bool,

    pub allow_drop_tables: bool,

    pub dry_run: bool,

    pub verbose: bool,
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            allow_drop_columns: false,
            allow_drop_tables:  false,
            dry_run:            false,
            verbose:            false,
        }
    }
}

pub struct TableSchema {
    table_name: &'static str,
    columns:    Vec<TableColumnInfo>,
}

#[derive(Debug, Clone)]
pub struct TableColumnInfo {
    pub name:              &'static str,
    pub column_type:       ColumnType,
    pub nullable:          bool,
    pub is_primary_key:    bool,
    pub is_auto_increment: bool,
    pub is_unique:         bool,
    pub default_value:     Option<&'static str>,

    pub renamed_from: Option<&'static str>,
    pub foreign_key:  Option<ForeignKeyInfo>,
}

impl TableSchema {
    pub fn of<Table: TableTrait>() -> Self
    where Table::Column: 'static {
        let columns = Table::Column::all()
            .iter()
            .map(|col| TableColumnInfo {
                name:              col.name(),
                column_type:       col.column_type(),
                nullable:          col.is_nullable(),
                is_primary_key:    col.is_primary_key(),
                is_auto_increment: col.is_auto_increment(),
                is_unique:         col.is_unique(),
                default_value:     col.default_value(),
                renamed_from:      col.renamed_from(),
                foreign_key:       col.foreign_key(),
            })
            .collect();

        Self { table_name: Table::table_name(), columns }
    }

    pub fn table_name(&self) -> &'static str {
        self.table_name
    }

    pub fn columns(&self) -> &[TableColumnInfo] {
        &self.columns
    }
}

pub struct Migrator;

impl Migrator {
    pub async fn migrate<Table: TableTrait>(conn: &crate::Connection) -> Result<SchemaDiff>
    where Table::Column: 'static {
        Self::migrate_with_options::<Table>(conn, MigrationOptions::default()).await
    }

    pub async fn migrate_with_options<Table: TableTrait>(
        conn: &crate::Connection,
        options: MigrationOptions,
    ) -> Result<SchemaDiff>
    where
        Table::Column: 'static,
    {
        let schema = TableSchema::of::<Table>();
        Self::migrate_schema(conn, &schema, &options).await
    }

    pub async fn migrate_all(conn: &crate::Connection, schemas: &[TableSchema]) -> Result<SchemaDiff> {
        Self::migrate_all_with_options(conn, schemas, MigrationOptions::default()).await
    }

    pub async fn migrate_all_with_options(
        conn: &crate::Connection,
        schemas: &[TableSchema],
        options: MigrationOptions,
    ) -> Result<SchemaDiff> {
        let mut combined_diff = SchemaDiff::empty();

        for schema in schemas {
            let diff = Self::migrate_schema(conn, schema, &options).await?;
            combined_diff.changes.extend(diff.changes);
            combined_diff.has_changes |= diff.has_changes;
            combined_diff.has_warnings |= diff.has_warnings;
        }

        Ok(combined_diff)
    }

    pub async fn introspect_table(conn: &crate::Connection, table_name: &str) -> Result<Option<DbTableInfo>> {
        let exists_sql = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?";
        let mut rows = conn.query(exists_sql, [table_name]).await?;

        let exists = if let Some(row) = rows.next().await? {
            let value = row.get_value(0)?;
            matches!(value, turso::Value::Integer(n) if n > 0)
        } else {
            false
        };

        if !exists {
            return Ok(None);
        }

        let pragma_sql = format!("PRAGMA table_info({})", table_name);
        let mut rows = conn.query(&pragma_sql, ()).await?;

        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();

        while let Some(row) = rows.next().await? {
            let name = match row.get_value(1)? {
                turso::Value::Text(s) => s,
                _ => continue,
            };

            let col_type = match row.get_value(2)? {
                turso::Value::Text(s) => s,
                _ => String::new(),
            };

            let notnull = match row.get_value(3)? {
                turso::Value::Integer(n) => n != 0,
                _ => false,
            };

            let default_value = match row.get_value(4)? {
                turso::Value::Text(s) => Some(s),
                turso::Value::Null => None,
                _ => None,
            };

            let is_pk = match row.get_value(5)? {
                turso::Value::Integer(n) => n > 0,
                _ => false,
            };

            if is_pk {
                primary_keys.push(name.clone());
            }

            columns.push(DbColumnInfo {
                name,
                column_type: col_type,
                nullable: !notnull,
                default_value,
                is_primary_key: is_pk,
            });
        }

        Ok(Some(DbTableInfo { name: table_name.to_string(), columns, primary_keys }))
    }

    pub async fn diff<Table: TableTrait>(conn: &crate::Connection) -> Result<SchemaDiff>
    where Table::Column: 'static {
        let schema = TableSchema::of::<Table>();
        Self::diff_schema(conn, &schema, &MigrationOptions::default()).await
    }

    async fn diff_schema(
        conn: &crate::Connection,
        entity_schema: &TableSchema,
        options: &MigrationOptions,
    ) -> Result<SchemaDiff> {
        let mut diff = SchemaDiff::empty();
        let table_name = entity_schema.table_name();

        let db_table = Self::introspect_table(conn, table_name).await?;

        match db_table {
            None => {
                let sql = Self::generate_create_table_sql(entity_schema);
                diff.add_change(SchemaChange::CreateTable { table_name: table_name.to_string(), sql });
            }
            Some(db_info) => {
                let db_columns: HashMap<&str, &DbColumnInfo> =
                    db_info.columns.iter().map(|c| (c.name.as_str(), c)).collect();

                let entity_columns: HashMap<&str, &TableColumnInfo> =
                    entity_schema.columns.iter().map(|c| (c.name, c)).collect();

                let mut renamed_old_columns: std::collections::HashSet<&str> = std::collections::HashSet::new();

                for entity_col in &entity_schema.columns {
                    if !db_columns.contains_key(entity_col.name) {
                        if let Some(old_name) = entity_col.renamed_from {
                            if db_columns.contains_key(old_name) {
                                let sql = format!(
                                    "ALTER TABLE {} RENAME COLUMN {} TO {}",
                                    table_name, old_name, entity_col.name
                                );
                                diff.add_change(SchemaChange::RenameColumn {
                                    table_name: table_name.to_string(),
                                    old_name: old_name.to_string(),
                                    new_name: entity_col.name.to_string(),
                                    sql,
                                });
                                renamed_old_columns.insert(old_name);
                            } else {
                                let sql = Self::generate_add_column_sql(table_name, entity_col);
                                diff.add_change(SchemaChange::AddColumn {
                                    table_name: table_name.to_string(),
                                    column_name: entity_col.name.to_string(),
                                    sql,
                                });
                            }
                        } else {
                            let sql = Self::generate_add_column_sql(table_name, entity_col);
                            diff.add_change(SchemaChange::AddColumn {
                                table_name: table_name.to_string(),
                                column_name: entity_col.name.to_string(),
                                sql,
                            });
                        }
                    } else {
                        let db_col = db_columns[entity_col.name];
                        if let Some(warning) = Self::check_column_compatibility(entity_col, db_col) {
                            diff.add_change(SchemaChange::Warning {
                                table_name: table_name.to_string(),
                                message:    warning,
                            });
                        }
                    }
                }

                for db_col in &db_info.columns {
                    if !entity_columns.contains_key(db_col.name.as_str())
                        && !renamed_old_columns.contains(db_col.name.as_str())
                    {
                        if options.allow_drop_columns {
                            let sql = format!("ALTER TABLE {} DROP COLUMN {}", table_name, db_col.name);
                            diff.add_change(SchemaChange::DropColumn {
                                table_name: table_name.to_string(),
                                column_name: db_col.name.clone(),
                                sql,
                            });
                        } else {
                            diff.add_change(SchemaChange::Warning {
                                table_name: table_name.to_string(),
                                message:    format!(
                                    "Column '{}' exists in database but not in entity definition",
                                    db_col.name
                                ),
                            });
                        }
                    }
                }

                if !conn.is_mvcc_enabled() {
                    for entity_col in &entity_schema.columns {
                        if entity_col.is_unique && !entity_col.is_primary_key {
                            let index_name = format!("idx_{}_{}_unique", table_name, entity_col.name);
                            let has_index = Self::index_exists(conn, &index_name).await?;

                            if !has_index {
                                let sql = format!(
                                    "CREATE UNIQUE INDEX IF NOT EXISTS {} ON {} ({})",
                                    index_name, table_name, entity_col.name
                                );
                                diff.add_change(SchemaChange::CreateIndex {
                                    table_name: table_name.to_string(),
                                    index_name,
                                    sql,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(diff)
    }

    async fn migrate_schema(
        conn: &crate::Connection,
        entity_schema: &TableSchema,
        options: &MigrationOptions,
    ) -> Result<SchemaDiff> {
        let diff = Self::diff_schema(conn, entity_schema, options).await?;

        if options.dry_run {
            return Ok(diff);
        }

        conn.execute("PRAGMA foreign_keys = OFF", ()).await?;

        for change in &diff.changes {
            if options.verbose {
                eprintln!("Migration: {}", change.description());
            }

            for sql in change.sql_statements() {
                if options.verbose {
                    eprintln!("  SQL: {}", sql);
                }
                conn.execute(sql, ()).await?;
            }
        }

        conn.execute("PRAGMA foreign_keys = ON", ()).await?;

        Ok(diff)
    }

    async fn index_exists(conn: &crate::Connection, index_name: &str) -> Result<bool> {
        let sql = "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?";
        let mut rows = conn.query(sql, [index_name]).await?;

        if let Some(row) = rows.next().await? {
            let value = row.get_value(0)?;
            Ok(matches!(value, turso::Value::Integer(n) if n > 0))
        } else {
            Ok(false)
        }
    }

    fn generate_create_table_sql(schema: &TableSchema) -> String {
        let mut column_defs = Vec::new();
        let mut primary_keys = Vec::new();

        for col in &schema.columns {
            let mut def = format!("{} {}", col.name, column_type_to_sql(col.column_type));

            if col.is_primary_key {
                primary_keys.push(col.name);
                if col.is_auto_increment {
                    def.push_str(" PRIMARY KEY AUTOINCREMENT");
                }
            }

            if !col.nullable && !col.is_primary_key {
                def.push_str(" NOT NULL");
            }

            if col.is_unique && !col.is_primary_key {
                def.push_str(" UNIQUE");
            }

            if let Some(default) = col.default_value {
                def.push_str(&format!(" DEFAULT {}", default));
            }

            column_defs.push(def);
        }

        let has_inline_pk = schema.columns.iter().any(|c| c.is_primary_key && c.is_auto_increment);

        if !has_inline_pk && !primary_keys.is_empty() {
            column_defs.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
        }

        column_defs.extend(Self::generate_create_foreign_key_changes(schema));

        format!("CREATE TABLE {} ({})", schema.table_name, column_defs.join(", "))
    }

    fn generate_create_foreign_key_changes(schema: &TableSchema) -> Vec<String> {
        schema
            .columns
            .iter()
            .filter(|col| col.foreign_key.is_some())
            .map(Self::generate_create_foreign_key_sql_from_column)
            .collect()
    }

    fn generate_create_foreign_key_sql_from_column(col: &TableColumnInfo) -> String {
        let foreign_key_info = col.foreign_key.as_ref().unwrap();
        let base_sql = format!("FOREIGN KEY ({}) REFERENCES {}", col.name, foreign_key_info.table_name);

        // Not yet implemented, ignored
        let _on_delete = match foreign_key_info.on_delete {
            OnDelete::Restrict => "ON DELETE RESTRICT",
            OnDelete::Cascade => "ON DELETE CASCADE",
            OnDelete::SetNull => " ON DELETE SET NULL",
            OnDelete::SetDefault => " ON DELETE SET DEFAULT",
            OnDelete::None => "",
        };

        // Not yet implemented, ignored
        let _on_update = match foreign_key_info.on_update {
            OnUpdate::Restrict => "ON UPDATE RESTRICT",
            OnUpdate::Cascade => "ON UPDATE CASCADE",
            OnUpdate::SetNull => "ON UPDATE SET NULL",
            OnUpdate::SetDefault => "ON UPDATE SET DEFAULT",
            OnUpdate::None => "",
        };

        // format!("{} {} {}", base_sql, on_delete, on_update)
        base_sql
    }

    fn generate_add_column_sql(table_name: &str, col: &TableColumnInfo) -> String {
        let mut def =
            format!("ALTER TABLE {} ADD COLUMN {} {}", table_name, col.name, column_type_to_sql(col.column_type));

        if !col.nullable {
            if let Some(default) = col.default_value {
                def.push_str(&format!(" NOT NULL DEFAULT {}", default));
            } else {
                let default = match col.column_type {
                    ColumnType::Integer => "0",
                    ColumnType::Float => "0.0",
                    ColumnType::Text => "''",
                    ColumnType::Blob => "X''",
                    ColumnType::Null => "NULL",
                };
                def.push_str(&format!(" NOT NULL DEFAULT {}", default));
            }
        } else if let Some(default) = col.default_value {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        def
    }

    fn check_column_compatibility(entity_col: &TableColumnInfo, db_col: &DbColumnInfo) -> Option<String> {
        let entity_type = column_type_to_sql(entity_col.column_type).to_uppercase();
        let db_type = db_col.column_type.to_uppercase();

        let type_compatible = match (entity_type.as_str(), db_type.as_str()) {
            ("INTEGER", "INTEGER") => true,
            ("INTEGER", "INT") => true,
            ("REAL", "REAL") => true,
            ("REAL", "FLOAT") => true,
            ("REAL", "DOUBLE") => true,
            ("TEXT", "TEXT") => true,
            ("TEXT", "VARCHAR") => true,
            ("TEXT", s) if s.starts_with("VARCHAR") => true,
            ("BLOB", "BLOB") => true,
            _ => entity_type == db_type,
        };

        if !type_compatible {
            return Some(format!(
                "Column '{}' type mismatch: entity expects {}, database has {}",
                entity_col.name, entity_type, db_type
            ));
        }

        if entity_col.nullable != db_col.nullable && !entity_col.is_primary_key {
            return Some(format!(
                "Column '{}' nullability mismatch: entity is {}, database is {}",
                entity_col.name,
                if entity_col.nullable { "nullable" } else { "NOT NULL" },
                if db_col.nullable { "nullable" } else { "NOT NULL" }
            ));
        }

        None
    }
}

fn column_type_to_sql(col_type: ColumnType) -> &'static str {
    match col_type {
        ColumnType::Integer => "INTEGER",
        ColumnType::Float => "REAL",
        ColumnType::Text => "TEXT",
        ColumnType::Blob => "BLOB",
        ColumnType::Null => "NULL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_change_description_create_table() {
        let change = SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        };
        assert_eq!(change.description(), "Create table 'users'");
    }

    #[test]
    fn test_schema_change_description_add_column() {
        let change = SchemaChange::AddColumn {
            table_name:  "users".to_string(),
            column_name: "email".to_string(),
            sql:         "ALTER TABLE users ADD COLUMN email TEXT".to_string(),
        };
        assert_eq!(change.description(), "Add column 'email' to table 'users'");
    }

    #[test]
    fn test_schema_change_description_drop_column() {
        let change = SchemaChange::DropColumn {
            table_name:  "users".to_string(),
            column_name: "old_field".to_string(),
            sql:         "ALTER TABLE users DROP COLUMN old_field".to_string(),
        };
        assert_eq!(change.description(), "Drop column 'old_field' from table 'users'");
    }

    #[test]
    fn test_schema_change_description_recreate_table() {
        let change = SchemaChange::RecreateTable {
            table_name: "users".to_string(),
            reason:     "change column type".to_string(),
            sql:        vec!["SQL1".to_string(), "SQL2".to_string()],
        };
        assert_eq!(change.description(), "Recreate table 'users': change column type");
    }

    #[test]
    fn test_schema_change_description_create_index() {
        let change = SchemaChange::CreateIndex {
            table_name: "users".to_string(),
            index_name: "idx_users_email".to_string(),
            sql:        "CREATE INDEX idx_users_email ON users (email)".to_string(),
        };
        assert_eq!(change.description(), "Create index 'idx_users_email' on table 'users'");
    }

    #[test]
    fn test_schema_change_description_warning() {
        let change =
            SchemaChange::Warning { table_name: "users".to_string(), message: "Column type mismatch".to_string() };
        assert_eq!(change.description(), "Warning for 'users': Column type mismatch");
    }

    #[test]
    fn test_schema_change_sql_create_table() {
        let change = SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        };
        let stmts = change.sql_statements();
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "CREATE TABLE users (id INTEGER)");
    }

    #[test]
    fn test_schema_change_sql_add_column() {
        let change = SchemaChange::AddColumn {
            table_name:  "users".to_string(),
            column_name: "email".to_string(),
            sql:         "ALTER TABLE users ADD COLUMN email TEXT".to_string(),
        };
        let stmts = change.sql_statements();
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "ALTER TABLE users ADD COLUMN email TEXT");
    }

    #[test]
    fn test_schema_change_sql_recreate_table() {
        let change = SchemaChange::RecreateTable {
            table_name: "users".to_string(),
            reason:     "test".to_string(),
            sql:        vec![
                "CREATE TABLE users_new (...)".to_string(),
                "INSERT INTO users_new SELECT * FROM users".to_string(),
                "DROP TABLE users".to_string(),
                "ALTER TABLE users_new RENAME TO users".to_string(),
            ],
        };
        let stmts = change.sql_statements();
        assert_eq!(stmts.len(), 4);
    }

    #[test]
    fn test_schema_change_sql_warning() {
        let change = SchemaChange::Warning { table_name: "users".to_string(), message: "test warning".to_string() };
        let stmts = change.sql_statements();
        assert!(stmts.is_empty());
    }

    #[test]
    fn test_db_column_info_equality() {
        let col1 = DbColumnInfo {
            name:           "email".to_string(),
            column_type:    "TEXT".to_string(),
            nullable:       true,
            default_value:  None,
            is_primary_key: false,
        };
        let col2 = col1.clone();
        assert_eq!(col1, col2);
    }

    #[test]
    fn test_db_column_info_debug() {
        let col = DbColumnInfo {
            name:           "id".to_string(),
            column_type:    "INTEGER".to_string(),
            nullable:       false,
            default_value:  None,
            is_primary_key: true,
        };
        let debug = format!("{:?}", col);
        assert!(debug.contains("id"));
        assert!(debug.contains("INTEGER"));
        assert!(debug.contains("is_primary_key: true"));
    }

    #[test]
    fn test_db_table_info_clone() {
        let table = DbTableInfo {
            name:         "users".to_string(),
            columns:      vec![DbColumnInfo {
                name:           "id".to_string(),
                column_type:    "INTEGER".to_string(),
                nullable:       false,
                default_value:  None,
                is_primary_key: true,
            }],
            primary_keys: vec!["id".to_string()],
        };
        let cloned = table.clone();
        assert_eq!(cloned.name, "users");
        assert_eq!(cloned.columns.len(), 1);
    }

    #[test]
    fn test_schema_diff_empty() {
        let diff = SchemaDiff::empty();
        assert!(!diff.has_changes);
        assert!(!diff.has_warnings);
        assert!(diff.changes.is_empty());
    }

    #[test]
    fn test_schema_diff_add_change() {
        let mut diff = SchemaDiff::empty();
        diff.add_change(SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        });

        assert!(diff.has_changes);
        assert!(!diff.has_warnings);
        assert_eq!(diff.changes.len(), 1);
    }

    #[test]
    fn test_schema_diff_add_warning() {
        let mut diff = SchemaDiff::empty();
        diff.add_change(SchemaChange::Warning {
            table_name: "users".to_string(),
            message:    "test warning".to_string(),
        });

        assert!(!diff.has_changes);
        assert!(diff.has_warnings);
        assert_eq!(diff.changes.len(), 1);
    }

    #[test]
    fn test_schema_diff_all_sql() {
        let mut diff = SchemaDiff::empty();
        diff.add_change(SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        });
        diff.add_change(SchemaChange::AddColumn {
            table_name:  "users".to_string(),
            column_name: "email".to_string(),
            sql:         "ALTER TABLE users ADD COLUMN email TEXT".to_string(),
        });

        let all_sql = diff.all_sql();
        assert_eq!(all_sql.len(), 2);
    }

    #[test]
    fn test_schema_diff_summary_empty() {
        let diff = SchemaDiff::empty();
        assert_eq!(diff.summary(), "No changes needed");
    }

    #[test]
    fn test_schema_diff_summary_with_changes() {
        let mut diff = SchemaDiff::empty();
        diff.add_change(SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        });

        let summary = diff.summary();
        assert!(summary.contains("Create table 'users'"));
    }

    #[test]
    fn test_schema_diff_summary_multiple_changes() {
        let mut diff = SchemaDiff::empty();
        diff.add_change(SchemaChange::CreateTable {
            table_name: "users".to_string(),
            sql:        "CREATE TABLE users (id INTEGER)".to_string(),
        });
        diff.add_change(SchemaChange::AddColumn {
            table_name:  "users".to_string(),
            column_name: "email".to_string(),
            sql:         "ALTER TABLE users ADD COLUMN email TEXT".to_string(),
        });

        let summary = diff.summary();
        assert!(summary.contains("Create table 'users'"));
        assert!(summary.contains("Add column 'email'"));
        assert!(summary.contains("\n"));
    }

    #[test]
    fn test_migration_options_default() {
        let opts = MigrationOptions::default();
        assert!(!opts.allow_drop_columns);
        assert!(!opts.allow_drop_tables);
        assert!(!opts.dry_run);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_migration_options_clone() {
        let opts = MigrationOptions {
            allow_drop_columns: true,
            allow_drop_tables:  true,
            dry_run:            true,
            verbose:            true,
        };
        let cloned = opts.clone();
        assert!(cloned.allow_drop_columns);
        assert!(cloned.allow_drop_tables);
        assert!(cloned.dry_run);
        assert!(cloned.verbose);
    }

    #[test]
    fn test_migration_options_debug() {
        let opts = MigrationOptions::default();
        let debug = format!("{:?}", opts);
        assert!(debug.contains("MigrationOptions"));
        assert!(debug.contains("allow_drop_columns"));
    }

    #[test]
    fn test_entity_column_info_clone() {
        let col = TableColumnInfo {
            name:              "id",
            column_type:       ColumnType::Integer,
            nullable:          false,
            is_primary_key:    true,
            is_auto_increment: true,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let cloned = col.clone();
        assert_eq!(cloned.name, "id");
        assert!(cloned.is_primary_key);
    }

    #[test]
    fn test_entity_column_info_debug() {
        let col = TableColumnInfo {
            name:              "email",
            column_type:       ColumnType::Text,
            nullable:          true,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         true,
            default_value:     Some("''"),
            renamed_from:      None,
            foreign_key:       None,
        };
        let debug = format!("{:?}", col);
        assert!(debug.contains("email"));
        assert!(debug.contains("is_unique: true"));
    }

    #[test]
    fn test_column_type_to_sql() {
        assert_eq!(column_type_to_sql(ColumnType::Integer), "INTEGER");
        assert_eq!(column_type_to_sql(ColumnType::Float), "REAL");
        assert_eq!(column_type_to_sql(ColumnType::Text), "TEXT");
        assert_eq!(column_type_to_sql(ColumnType::Blob), "BLOB");
        assert_eq!(column_type_to_sql(ColumnType::Null), "NULL");
    }

    #[test]
    fn test_generate_create_table_sql_basic() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![
                TableColumnInfo {
                    name:              "id",
                    column_type:       ColumnType::Integer,
                    nullable:          false,
                    is_primary_key:    true,
                    is_auto_increment: true,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
                TableColumnInfo {
                    name:              "name",
                    column_type:       ColumnType::Text,
                    nullable:          false,
                    is_primary_key:    false,
                    is_auto_increment: false,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
            ],
        };

        let sql = Migrator::generate_create_table_sql(&schema);
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("name TEXT NOT NULL"));
    }

    #[test]
    fn test_generate_create_table_sql_with_unique() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![
                TableColumnInfo {
                    name:              "id",
                    column_type:       ColumnType::Integer,
                    nullable:          false,
                    is_primary_key:    true,
                    is_auto_increment: true,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
                TableColumnInfo {
                    name:              "email",
                    column_type:       ColumnType::Text,
                    nullable:          false,
                    is_primary_key:    false,
                    is_auto_increment: false,
                    is_unique:         true,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
            ],
        };

        let sql = Migrator::generate_create_table_sql(&schema);
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    }

    #[test]
    fn test_generate_create_table_sql_with_default() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![
                TableColumnInfo {
                    name:              "id",
                    column_type:       ColumnType::Integer,
                    nullable:          false,
                    is_primary_key:    true,
                    is_auto_increment: true,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
                TableColumnInfo {
                    name:              "status",
                    column_type:       ColumnType::Text,
                    nullable:          false,
                    is_primary_key:    false,
                    is_auto_increment: false,
                    is_unique:         false,
                    default_value:     Some("'active'"),
                    renamed_from:      None,
                    foreign_key:       None,
                },
            ],
        };

        let sql = Migrator::generate_create_table_sql(&schema);
        assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
    }

    #[test]
    fn test_generate_create_table_sql_nullable() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![
                TableColumnInfo {
                    name:              "id",
                    column_type:       ColumnType::Integer,
                    nullable:          false,
                    is_primary_key:    true,
                    is_auto_increment: true,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
                TableColumnInfo {
                    name:              "bio",
                    column_type:       ColumnType::Text,
                    nullable:          true,
                    is_primary_key:    false,
                    is_auto_increment: false,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
            ],
        };

        let sql = Migrator::generate_create_table_sql(&schema);
        assert!(sql.contains("bio TEXT"));
        assert!(!sql.contains("bio TEXT NOT NULL"));
    }

    #[test]
    fn test_generate_create_table_sql_non_auto_pk() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![TableColumnInfo {
                name:              "id",
                column_type:       ColumnType::Integer,
                nullable:          false,
                is_primary_key:    true,
                is_auto_increment: false,
                is_unique:         false,
                default_value:     None,
                renamed_from:      None,
                foreign_key:       None,
            }],
        };

        let sql = Migrator::generate_create_table_sql(&schema);
        assert!(sql.contains("PRIMARY KEY (id)"));
        assert!(!sql.contains("AUTOINCREMENT"));
    }

    #[test]
    fn test_generate_add_column_sql_not_null_with_default() {
        let col = TableColumnInfo {
            name:              "status",
            column_type:       ColumnType::Text,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     Some("'active'"),
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("users", &col);
        assert!(sql.contains("ALTER TABLE users ADD COLUMN status TEXT NOT NULL DEFAULT 'active'"));
    }

    #[test]
    fn test_generate_add_column_sql_not_null_without_default() {
        let col = TableColumnInfo {
            name:              "name",
            column_type:       ColumnType::Text,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("users", &col);

        assert!(sql.contains("ALTER TABLE users ADD COLUMN name TEXT NOT NULL DEFAULT ''"));
    }

    #[test]
    fn test_generate_add_column_sql_nullable() {
        let col = TableColumnInfo {
            name:              "bio",
            column_type:       ColumnType::Text,
            nullable:          true,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("users", &col);
        assert!(sql.contains("ALTER TABLE users ADD COLUMN bio TEXT"));
        assert!(!sql.contains("NOT NULL"));
    }

    #[test]
    fn test_generate_add_column_sql_integer_default() {
        let col = TableColumnInfo {
            name:              "count",
            column_type:       ColumnType::Integer,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("stats", &col);
        assert!(sql.contains("DEFAULT 0"));
    }

    #[test]
    fn test_generate_add_column_sql_float_default() {
        let col = TableColumnInfo {
            name:              "rating",
            column_type:       ColumnType::Float,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("products", &col);
        assert!(sql.contains("DEFAULT 0.0"));
    }

    #[test]
    fn test_generate_add_column_sql_blob_default() {
        let col = TableColumnInfo {
            name:              "data",
            column_type:       ColumnType::Blob,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };

        let sql = Migrator::generate_add_column_sql("files", &col);
        assert!(sql.contains("DEFAULT X''"));
    }

    #[test]
    fn test_check_column_compatibility_same_type() {
        let entity_col = TableColumnInfo {
            name:              "id",
            column_type:       ColumnType::Integer,
            nullable:          false,
            is_primary_key:    true,
            is_auto_increment: true,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let db_col = DbColumnInfo {
            name:           "id".to_string(),
            column_type:    "INTEGER".to_string(),
            nullable:       false,
            default_value:  None,
            is_primary_key: true,
        };

        let result = Migrator::check_column_compatibility(&entity_col, &db_col);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_column_compatibility_type_mismatch() {
        let entity_col = TableColumnInfo {
            name:              "age",
            column_type:       ColumnType::Integer,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let db_col = DbColumnInfo {
            name:           "age".to_string(),
            column_type:    "TEXT".to_string(),
            nullable:       false,
            default_value:  None,
            is_primary_key: false,
        };

        let result = Migrator::check_column_compatibility(&entity_col, &db_col);
        assert!(result.is_some());
        assert!(result.unwrap().contains("type mismatch"));
    }

    #[test]
    fn test_check_column_compatibility_nullable_mismatch() {
        let entity_col = TableColumnInfo {
            name:              "email",
            column_type:       ColumnType::Text,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let db_col = DbColumnInfo {
            name:           "email".to_string(),
            column_type:    "TEXT".to_string(),
            nullable:       true,
            default_value:  None,
            is_primary_key: false,
        };

        let result = Migrator::check_column_compatibility(&entity_col, &db_col);
        assert!(result.is_some());
        assert!(result.unwrap().contains("nullability mismatch"));
    }

    #[test]
    fn test_check_column_compatibility_compatible_types() {
        let entity_col = TableColumnInfo {
            name:              "id",
            column_type:       ColumnType::Integer,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let db_col = DbColumnInfo {
            name:           "id".to_string(),
            column_type:    "INT".to_string(),
            nullable:       false,
            default_value:  None,
            is_primary_key: false,
        };

        let result = Migrator::check_column_compatibility(&entity_col, &db_col);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_column_compatibility_varchar() {
        let entity_col = TableColumnInfo {
            name:              "name",
            column_type:       ColumnType::Text,
            nullable:          false,
            is_primary_key:    false,
            is_auto_increment: false,
            is_unique:         false,
            default_value:     None,
            renamed_from:      None,
            foreign_key:       None,
        };
        let db_col = DbColumnInfo {
            name:           "name".to_string(),
            column_type:    "VARCHAR(255)".to_string(),
            nullable:       false,
            default_value:  None,
            is_primary_key: false,
        };

        let result = Migrator::check_column_compatibility(&entity_col, &db_col);
        assert!(result.is_none());
    }

    #[test]
    fn test_entity_schema_table_name() {
        let schema = TableSchema { table_name: "my_table", columns: vec![] };
        assert_eq!(schema.table_name(), "my_table");
    }

    #[test]
    fn test_entity_schema_columns() {
        let schema = TableSchema {
            table_name: "users",
            columns:    vec![
                TableColumnInfo {
                    name:              "id",
                    column_type:       ColumnType::Integer,
                    nullable:          false,
                    is_primary_key:    true,
                    is_auto_increment: true,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
                TableColumnInfo {
                    name:              "name",
                    column_type:       ColumnType::Text,
                    nullable:          false,
                    is_primary_key:    false,
                    is_auto_increment: false,
                    is_unique:         false,
                    default_value:     None,
                    renamed_from:      None,
                    foreign_key:       None,
                },
            ],
        };

        assert_eq!(schema.columns().len(), 2);
        assert_eq!(schema.columns()[0].name, "id");
        assert_eq!(schema.columns()[1].name, "name");
    }

    #[test]
    fn test_schema_change_description_rename_column() {
        let change = SchemaChange::RenameColumn {
            table_name: "users".to_string(),
            old_name:   "timestamp".to_string(),
            new_name:   "created_at".to_string(),
            sql:        "ALTER TABLE users RENAME COLUMN timestamp TO created_at".to_string(),
        };
        assert_eq!(change.description(), "Rename column 'timestamp' to 'created_at' in table 'users'");
    }

    #[test]
    fn test_schema_change_sql_rename_column() {
        let change = SchemaChange::RenameColumn {
            table_name: "users".to_string(),
            old_name:   "timestamp".to_string(),
            new_name:   "created_at".to_string(),
            sql:        "ALTER TABLE users RENAME COLUMN timestamp TO created_at".to_string(),
        };
        let stmts = change.sql_statements();
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "ALTER TABLE users RENAME COLUMN timestamp TO created_at");
    }
}
