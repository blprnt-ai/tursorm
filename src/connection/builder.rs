pub struct Builder {
    pub(super) path:              String,
    pub(super) enable_mvcc:       bool,
    pub(super) enable_encryption: bool,
    pub(super) vfs:               Option<String>,
    pub(super) encryption_opts:   Option<turso::EncryptionOpts>,
}

impl Builder {
    pub fn new_local(path: &str) -> Self {
        Self {
            path:              path.to_string(),
            enable_mvcc:       false,
            enable_encryption: false,
            vfs:               None,
            encryption_opts:   None,
        }
    }

    pub fn with_mvcc(mut self, mvcc: bool) -> Self {
        self.enable_mvcc = mvcc;
        self
    }

    pub fn experimental_encryption(mut self, encryption_enabled: bool) -> Self {
        self.enable_encryption = encryption_enabled;
        self
    }

    pub fn with_encryption(mut self, opts: turso::EncryptionOpts) -> Self {
        self.encryption_opts = Some(opts);
        self
    }

    pub fn with_io(mut self, vfs: String) -> Self {
        self.vfs = Some(vfs);
        self
    }

    pub async fn build(self) -> super::ConnectionResult<super::database::Database> {
        let opts = super::opts::DatabaseOpts::from(&self);

        let mut turso_builder = turso::Builder::new_local(&self.path);
        turso_builder = turso_builder.with_mvcc(self.enable_mvcc);
        turso_builder = turso_builder.experimental_encryption(self.enable_encryption);

        turso_builder = match self.encryption_opts {
            Some(opts) => turso_builder.with_encryption(opts),
            None => turso_builder,
        };
        turso_builder = match self.vfs {
            Some(vfs) => turso_builder.with_io(vfs),
            None => turso_builder,
        };

        let db = turso_builder.build().await?;

        Ok(super::database::Database::new(db, opts))
    }
}
