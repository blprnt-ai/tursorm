use super::table::TableTrait;
use crate::Condition;
use crate::Delete;
use crate::value::Value;

pub trait RecordTrait: Clone + Send + Sync {
    type Table: TableTrait;

    fn get_primary_key_value(&self) -> Value;

    fn into_change_set(self) -> <Self::Table as TableTrait>::ChangeSet
    where <Self::Table as TableTrait>::ChangeSet: From<Self> {
        <Self::Table as TableTrait>::ChangeSet::from(self)
    }
}

pub trait RecordDeleteExt: RecordTrait {
    fn delete(self) -> Delete<Self::Table> {
        Delete::new().filter(Condition::eq(Self::Table::primary_key(), self.get_primary_key_value()))
    }
}
impl<Record: RecordTrait> RecordDeleteExt for Record {}
