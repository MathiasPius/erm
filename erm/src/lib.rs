#![feature(macro_metavar_expr)]

mod archetype;
pub mod backend;
mod component;
pub mod condition;
pub mod cte;
mod entity;
mod reflect;
pub mod row;

pub use archetype::Archetype;
pub use component::{ColumnDefinition, Component};
pub use entity::EntityPrefixedQuery;
pub use reflect::{Reflect, ReflectedColumn};
pub use row::OffsetRow;

#[cfg(feature = "derive")]
pub use erm_derive::*;
