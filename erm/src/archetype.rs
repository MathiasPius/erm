use crate::{
    component::{Component, ComponentDesc},
    select::{Column, Compound, Join},
};

pub trait Archetype: Sized {
    const COMPONENTS: &'static [ComponentDesc];

    fn as_description() -> ArchetypeDesc {
        ArchetypeDesc {
            components: Self::COMPONENTS,
        }
    }
}

pub struct ArchetypeDesc {
    pub components: &'static [ComponentDesc],
}

impl From<&ArchetypeDesc> for Compound {
    fn from(value: &ArchetypeDesc) -> Self {
        let mut components = value.components.iter();
        let first = components.next().expect("empty archetype");

        let joins = components
            .map(|component| Join {
                table: component.into(),
                columns: (
                    Column {
                        table: first.table_name,
                        name: "entity",
                    },
                    Column {
                        table: component.table_name,
                        name: "entity",
                    },
                ),
            })
            .collect();

        Compound {
            source: first.into(),
            joins,
        }
    }
}

macro_rules! impl_archetype_tuple {
    ($head:ident, $($tail:ident),*) => {
        impl<$head, $($tail),*> Archetype for ($head, $($tail),*)
    where
        $head: Component,
        $($tail: Component),*
    {
        const COMPONENTS: &'static [ComponentDesc] = &[
            $head::DESCRIPTION,
            $($tail::DESCRIPTION,)*
        ];
    }
    };
}

macro_rules! impl_recursive_archetype_tuple {
    ($head:ident) => {
        impl_archetype_tuple!($head,);
    };
    ($head:ident, $($tail:ident),+) => {
        impl_archetype_tuple!($head, $($tail),+);
        impl_recursive_archetype_tuple!($($tail),*);
    };
}

impl_recursive_archetype_tuple!(
    C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15, C16
);
