use std::marker::PhantomData;

use crate::condition::{Equality, Inequality};

pub trait Reflect {
    type ReflectionType;
    const FIELDS: Self::ReflectionType;
}

#[derive(Debug, Clone, Copy)]
pub struct ReflectedColumn<T> {
    column_name: &'static str,
    _data: PhantomData<T>,
}

impl<T> ReflectedColumn<T> {
    pub const fn new(column_name: &'static str) -> Self {
        Self {
            column_name,
            _data: PhantomData,
        }
    }
}

impl<T> ReflectedColumn<T> {
    pub const fn eq(&self, value: T) -> Equality<T> {
        Equality::new(self.column_name, value)
    }
    pub const fn ne(&self, value: T) -> Inequality<T> {
        Inequality::new(self.column_name, value)
    }
}
