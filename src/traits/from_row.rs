use crate::error::Result;

pub trait FromRow: Sized {
    fn from_row(row: &turso::Row) -> Result<Self>;
}
