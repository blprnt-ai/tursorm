use super::table::TableTrait;
use crate::IntoValue;
use crate::error::Result;
use crate::traits::column::RowIdColumn;
use crate::value::Value;

#[async_trait::async_trait]
pub trait ChangeSetTrait: std::fmt::Debug + Default + Clone + Send + Sync + Sized + 'static {
    type Table: TableTrait<ChangeSet = Self>;

    fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>);

    fn get_update_sets(&self) -> Vec<(&'static str, Value)>;

    fn get_primary_key_value(&self) -> Option<Value>;

    fn primary_key_column() -> &'static str;

    #[tracing::instrument(skip(self, conn))]
    async fn insert(self, conn: &crate::Connection) -> Result<<Self::Table as TableTrait>::Record>
    where <Self::Table as TableTrait>::Record: Send {
        tracing::trace!("Inserting record");

        let db_row_id = crate::query::Insert::<Self::Table>::new(self).exec_with_last_insert_id(conn).await?;

        let row = crate::query::Select::<Self::Table>::new()
            .filter(crate::query::Condition::eq(RowIdColumn, db_row_id.into_value()))
            .one(conn)
            .await?;

        tracing::trace!("Row: {:?}", row);
        row.ok_or(crate::error::Error::NoRowsAffected)
    }

    #[tracing::instrument(skip(self, conn))]
    async fn insert_exec(self, conn: &crate::Connection) -> Result<u64> {
        tracing::trace!("Inserting record");
        let affected = crate::query::Insert::<Self::Table>::new(self).exec(conn).await?;

        tracing::trace!("Affected: {}", affected);
        Ok(affected)
    }

    #[tracing::instrument(skip(self, conn))]
    async fn update(self, conn: &crate::Connection) -> Result<<Self::Table as TableTrait>::Record>
    where <Self::Table as TableTrait>::Record: Send {
        let pk_value = self.get_primary_key_value().ok_or(crate::error::Error::PrimaryKeyNotSet)?;

        tracing::trace!("Updating record");
        crate::query::Update::<Self::Table>::new(self).exec(conn).await?;

        let record = crate::query::Select::<Self::Table>::new()
            .filter(crate::query::Condition::eq(Self::Table::primary_key(), pk_value))
            .one(conn)
            .await?;

        record.ok_or(crate::error::Error::NoRowsAffected)
    }

    #[tracing::instrument(skip(self, conn))]
    async fn update_exec(self, conn: &crate::Connection) -> Result<u64> {
        tracing::trace!("Updating record");
        let affected = crate::query::Update::<Self::Table>::new(self).exec(conn).await?;

        tracing::trace!("Affected: {}", affected);
        Ok(affected)
    }

    #[tracing::instrument(skip(self, conn))]
    async fn delete(self, conn: &crate::Connection) -> Result<u64> {
        let pk_value = self.get_primary_key_value().ok_or(crate::error::Error::PrimaryKeyNotSet)?;
        tracing::trace!("Deleting record");
        let affected = crate::query::Delete::<Self::Table>::new()
            .filter(crate::query::Condition::eq(Self::Table::primary_key(), pk_value))
            .exec(conn)
            .await?;

        tracing::trace!("Affected: {}", affected);
        Ok(affected)
    }
}
