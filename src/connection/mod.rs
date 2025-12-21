pub(crate) mod builder;
pub(crate) mod database;
pub(crate) mod opts;

pub mod prelude {
    pub use super::Connection;
    pub use super::builder::Builder;
}

pub(self) type ConnectionResult<T> = std::result::Result<T, turso::Error>;

#[derive(Debug, Clone)]
pub struct Connection {
    inner: turso::Connection,
    opts:  opts::DatabaseOpts,
}

impl Connection {
    fn new(inner: turso::Connection, opts: opts::DatabaseOpts) -> Self {
        Self { inner, opts }
    }

    // TODO: Investigate failures when using transactions
    //
    // PANIC:
    // turso_core-0.3.2/storage/wal.rs:986:13:
    // must have a read transaction to begin a write transaction
    // pub async fn begin(&mut self) -> turso::Result<turso::transaction::Transaction<'_>> {
    //     self.inner.transaction().await
    // }

    pub fn is_mvcc_enabled(&self) -> bool {
        self.opts.enable_mvcc
    }

    pub fn is_encryption_enabled(&self) -> bool {
        self.opts.enable_encryption
    }

    pub fn path(&self) -> &str {
        self.opts.path.as_str()
    }

    pub async fn query(&self, sql: &str, params: impl turso::IntoParams) -> turso::Result<turso::Rows> {
        self.inner.query(sql, params).await
    }

    pub async fn execute(&self, sql: &str, params: impl turso::IntoParams) -> turso::Result<u64> {
        self.inner.execute(sql, params).await
    }

    pub async fn execute_batch(&self, sql: &str) -> turso::Result<()> {
        self.inner.execute_batch(sql).await
    }

    pub async fn prepare(&self, sql: &str) -> turso::Result<turso::Statement> {
        self.inner.prepare(sql).await
    }

    pub async fn pragma_query(
        &self,
        pragma_name: &str,
        f: impl Fn(&turso::Row) -> std::result::Result<(), turso_core::LimboError>,
    ) -> turso::Result<()> {
        self.inner.pragma_query(pragma_name, f)
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.inner.last_insert_rowid()
    }

    pub fn cacheflush(&self) -> turso::Result<()> {
        self.inner.cacheflush()
    }

    pub fn is_autocommit(&self) -> turso::Result<bool> {
        self.inner.is_autocommit()
    }

    pub fn busy_timeout(&self, duration: std::time::Duration) -> turso::Result<()> {
        self.inner.busy_timeout(duration)
    }
}
