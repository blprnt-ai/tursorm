use super::change_set::ChangeSetTrait;
use super::column::ColumnTrait;
use super::from_row::FromRow;
use super::record::RecordTrait;
use crate::Condition;
use crate::Delete;
use crate::IntoValue;
use crate::Select;

pub trait TableTrait: std::fmt::Debug + Default + Send + Sync + 'static {
    type Record: RecordTrait<Table = Self> + FromRow + Send;

    type Column: ColumnTrait;

    type ChangeSet: ChangeSetTrait<Table = Self>;

    fn table_name() -> &'static str;

    fn primary_key() -> Self::Column;

    fn primary_key_auto_increment() -> bool;

    fn all_columns() -> &'static str;

    fn column_count() -> usize;
}

pub trait TableSelectExt: TableTrait {
    #[tracing::instrument]
    fn find() -> Select<Self> {
        Select::new()
    }

    #[tracing::instrument]
    fn find_by_id<V: crate::value::IntoValue>(id: V) -> Select<Self>
    where Self::Column: ColumnTrait {
        Select::new().filter(Condition::eq(<Self>::primary_key(), id))
    }
}

impl<Table: TableTrait> TableSelectExt for Table {}

pub trait TableDeleteExt: TableTrait {
    #[tracing::instrument]
    fn delete_many(records: Vec<Self::Record>) -> Delete<Self> {
        Delete::new()
            .filter(Condition::is_in(Self::primary_key(), records.iter().map(|m| m.get_primary_key_value()).collect()))
    }

    #[tracing::instrument]
    fn delete_many_by_ids<V: IntoValue>(ids: Vec<V>) -> Delete<Self> {
        Delete::new().filter(Condition::is_in(Self::primary_key(), ids))
    }

    #[tracing::instrument]
    fn truncate() -> Delete<Self> {
        Delete::new()
    }
}

impl<Table: TableTrait> TableDeleteExt for Table {}
