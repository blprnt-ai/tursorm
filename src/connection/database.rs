#[derive(Clone)]
pub struct Database {
    db:   turso::Database,
    opts: super::opts::DatabaseOpts,
}

impl Database {
    pub(super) fn new(db: turso::Database, opts: super::opts::DatabaseOpts) -> Self {
        Self { db, opts }
    }

    pub fn connect(self) -> super::ConnectionResult<super::Connection> {
        let conn = self.db.connect()?;
        Ok(super::Connection::new(conn, self.opts))
    }
}
