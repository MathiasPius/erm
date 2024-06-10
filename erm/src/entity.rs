use sqlx::{any::AnyTypeInfo, Any, Decode, Type};

use crate::{
    backend::Deserialize,
    component::{Component, Field},
};

#[derive(Debug)]
pub struct GenericEntity<T>(T);

impl<T> Component for GenericEntity<T> {
    const TABLE_NAME: &'static str = "erm_entities";

    const FIELDS: &'static [crate::component::Field] = &[Field {
        name: "entity",
        optional: false,
        type_info: AnyTypeInfo {
            kind: sqlx::postgres::any::AnyTypeInfoKind::Blob,
        },
    }];
}

impl<'r, T> Deserialize<'r> for GenericEntity<T>
where
    T: Decode<'r, sqlx::Any> + Type<Any>,
{
    fn deserialize(row: &'r crate::OffsetRow) -> Result<Self, sqlx::Error> {
        let inner: T = row.try_get(0)?;
        Ok(GenericEntity(inner))
    }
}
