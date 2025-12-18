use super::builder::Builder;

#[derive(Debug, Clone)]
pub struct DatabaseOpts {
    pub(super) path:              String,
    pub(super) enable_mvcc:       bool,
    pub(super) enable_encryption: bool,
}

impl From<&Builder> for DatabaseOpts {
    fn from(builder: &Builder) -> Self {
        Self {
            path:              builder.path.clone(),
            enable_mvcc:       builder.enable_mvcc.clone(),
            enable_encryption: builder.enable_encryption.clone(),
        }
    }
}
