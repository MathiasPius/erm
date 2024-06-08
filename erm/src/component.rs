use std::marker::PhantomData;

use sqlx::{any::AnyTypeInfo, Database, TypeInfo};

use crate::{
    create::{ColumnSpec, Create},
    insert::Insert,
    select::{Column, Select},
};

pub struct Field {
    pub name: &'static str,
    pub optional: bool,
    pub type_info: AnyTypeInfo,
}

pub struct ComponentDesc {
    pub table_name: &'static str,
    pub fields: &'static [Field],
}

pub trait Component: Sized {
    const TABLE_NAME: &'static str;
    const FIELDS: &'static [Field];

    const DESCRIPTION: ComponentDesc = ComponentDesc {
        table_name: Self::TABLE_NAME,
        fields: Self::FIELDS,
    };
}

impl From<&ComponentDesc> for Select {
    fn from(value: &ComponentDesc) -> Self {
        Select {
            table: value.table_name,
            columns: value
                .fields
                .iter()
                .map(|field| Column {
                    table: value.table_name,
                    name: field.name,
                })
                .collect(),
        }
    }
}

impl<DB: Database> From<&ComponentDesc> for Insert<DB> {
    fn from(value: &ComponentDesc) -> Self {
        Insert {
            table: value.table_name,
            values: value.fields.iter().map(|field| field.name).collect(),
            _db: std::marker::PhantomData,
        }
    }
}

impl<DB: Database> From<&ComponentDesc> for Create<DB> {
    fn from(value: &ComponentDesc) -> Self {
        Create {
            table: value.table_name,
            columns: value
                .fields
                .iter()
                .map(|field| ColumnSpec {
                    name: field.name,
                    r#type: field.type_info.name(),
                    null: field.optional,
                })
                .collect(),
            _db: PhantomData,
        }
    }
}
