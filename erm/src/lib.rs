#![feature(macro_metavar_expr)]

mod component;
pub mod cte;
mod offsets;

pub use component::{Archetype, Component, Get, Insert, List};
pub use offsets::OffsetRow;

#[cfg(feature = "derive")]
pub use erm_derive::*;
