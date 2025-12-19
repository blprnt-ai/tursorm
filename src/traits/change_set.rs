use super::table::TableTrait;
use crate::error::Result;
use crate::value::Value;

#[async_trait::async_trait]
pub trait ChangeSetTrait: Default + Clone + Send + Sync + Sized + 'static {
    type Table: TableTrait<ChangeSet = Self>;

    fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<Value>);

    fn get_update_sets(&self) -> Vec<(&'static str, Value)>;

    fn get_primary_key_value(&self) -> Option<Value>;

    fn primary_key_column() -> &'static str;

    async fn insert(self, conn: &crate::Connection) -> Result<<Self::Table as TableTrait>::Record>
    where <Self::Table as TableTrait>::Record: Send {
        let row_id = crate::query::Insert::<Self::Table>::new(self).exec_with_last_insert_id(conn).await?;
        let row = crate::query::Select::<Self::Table>::new()
            .filter(crate::query::Condition::eq(Self::Table::primary_key(), row_id))
            .one(conn)
            .await?;

        row.ok_or(crate::error::Error::NoRowsAffected)
    }

    async fn insert_exec(self, conn: &crate::Connection) -> Result<u64> {
        crate::query::Insert::<Self::Table>::new(self).exec(conn).await
    }

    async fn update(self, conn: &crate::Connection) -> Result<<Self::Table as TableTrait>::Record>
    where <Self::Table as TableTrait>::Record: Send {
        crate::query::Update::<Self::Table>::new(self).exec_with_returning(conn).await
    }

    async fn update_exec(self, conn: &crate::Connection) -> Result<u64> {
        crate::query::Update::<Self::Table>::new(self).exec(conn).await
    }

    async fn delete(self, conn: &crate::Connection) -> Result<u64> {
        let pk_value = self.get_primary_key_value().ok_or(crate::error::Error::PrimaryKeyNotSet)?;
        crate::query::Delete::<Self::Table>::new()
            .filter(crate::query::Condition::eq(Self::Table::primary_key(), pk_value))
            .exec(conn)
            .await
    }
}
