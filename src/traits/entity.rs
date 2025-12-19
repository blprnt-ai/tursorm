use super::active_model::ActiveModelTrait;
use super::column::ColumnTrait;
use super::from_row::FromRow;
use super::model::ModelTrait;
use crate::Condition;
use crate::Delete;
use crate::IntoValue;
use crate::Select;

pub trait EntityTrait: Default + Send + Sync + 'static {
    type Model: ModelTrait<Entity = Self> + FromRow + Send;

    type Column: ColumnTrait;

    type ActiveModel: ActiveModelTrait<Entity = Self>;

    fn table_name() -> &'static str;

    fn primary_key() -> Self::Column;

    fn primary_key_auto_increment() -> bool;

    fn all_columns() -> &'static str;

    fn column_count() -> usize;
}

pub trait EntitySelectExt: EntityTrait {
    fn find() -> Select<Self> {
        Select::new()
    }

    fn find_by_id<V: crate::value::IntoValue>(id: V) -> Select<Self>
    where Self::Column: ColumnTrait {
        Select::new().filter(Condition::eq(<Self>::primary_key(), id))
    }
}

impl<E: EntityTrait> EntitySelectExt for E {}

pub trait EntityDeleteExt: EntityTrait {
    fn delete_many(models: Vec<Self::Model>) -> Delete<Self> {
        Delete::new()
            .filter(Condition::is_in(Self::primary_key(), models.iter().map(|m| m.get_primary_key_value()).collect()))
    }

    fn delete_many_by_ids<V: IntoValue>(ids: Vec<V>) -> Delete<Self> {
        Delete::new().filter(Condition::is_in(Self::primary_key(), ids))
    }

    fn truncate() -> Delete<Self> {
        Delete::new()
    }
}

impl<E: EntityTrait> EntityDeleteExt for E {}
