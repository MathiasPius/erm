use std::marker::PhantomData;

use crate::condition::{
    Equality, GreaterThan, GreaterThanOrEqual, Inequality, LessThan, LessThanOrEqual,
};

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
    pub const fn gt(&self, value: T) -> GreaterThan<T> {
        GreaterThan::new(self.column_name, value)
    }
    pub const fn lt(&self, value: T) -> LessThan<T> {
        LessThan::new(self.column_name, value)
    }
    pub const fn ge(&self, value: T) -> GreaterThanOrEqual<T> {
        GreaterThanOrEqual::new(self.column_name, value)
    }
    pub const fn le(&self, value: T) -> LessThanOrEqual<T> {
        LessThanOrEqual::new(self.column_name, value)
    }

    pub const fn equals(&self, value: T) -> Equality<T> {
        self.eq(value)
    }
    pub const fn not_equals(&self, value: T) -> Inequality<T> {
        self.ne(value)
    }
    pub const fn greater_than(&self, value: T) -> GreaterThan<T> {
        self.gt(value)
    }
    pub const fn less_than(&self, value: T) -> LessThan<T> {
        self.lt(value)
    }
    pub const fn greater_than_or_equals(&self, value: T) -> GreaterThanOrEqual<T> {
        self.ge(value)
    }
    pub const fn less_than_or_equals(&self, value: T) -> LessThanOrEqual<T> {
        self.le(value)
    }
}
