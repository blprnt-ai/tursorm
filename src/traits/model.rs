use super::entity::EntityTrait;
use crate::Condition;
use crate::Delete;
use crate::value::Value;

pub trait ModelTrait: Clone + Send + Sync {
    type Entity: EntityTrait;

    fn get_primary_key_value(&self) -> Value;

    fn into_active_model(self) -> <Self::Entity as EntityTrait>::ActiveModel
    where <Self::Entity as EntityTrait>::ActiveModel: From<Self> {
        <Self::Entity as EntityTrait>::ActiveModel::from(self)
    }
}

pub trait ModelDeleteExt: ModelTrait {
    fn delete(self) -> Delete<Self::Entity> {
        Delete::new().filter(Condition::eq(Self::Entity::primary_key(), self.get_primary_key_value()))
    }
}
impl<M: ModelTrait> ModelDeleteExt for M {}
