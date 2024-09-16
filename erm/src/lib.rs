#![feature(macro_metavar_expr)]

pub mod archetype;
pub mod backend;
pub mod component;
pub mod condition;
pub mod cte;
pub mod entity;
pub mod reflect;
pub mod row;

pub mod prelude {
    #[cfg(feature = "derive")]
    pub use erm_derive::*;

    pub use crate::archetype::Archetype;
    pub use crate::backend::*;
    pub use crate::component::{ColumnDefinition, Component};
    pub use crate::condition;
    pub use crate::reflect::Reflect;
}
