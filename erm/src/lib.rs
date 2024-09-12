#![feature(macro_metavar_expr)]

mod archetype;
pub mod backend;
mod component;
mod condition;
pub mod cte;
mod entity;
pub mod row;

pub use archetype::Archetype;
pub use component::{ColumnDefinition, Component};
pub use entity::EntityPrefixedQuery;
pub use row::OffsetRow;

#[cfg(feature = "derive")]
pub use erm_derive::*;
