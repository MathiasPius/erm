use crate::{
    component::ComponentDesc,
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

/*
impl<C1> Archetype for (C1,)
where
    C1: Component,
{
    const COMPONENTS: &'static [ComponentDesc] = &[ComponentDesc {
        table_name: <C1 as Component>::TABLE_NAME,
        fields: <C1 as Component>::FIELDS,
    }];

    fn from_row(row: OffsetRow) -> Result<Self, sqlx::Error> {}
}
 */
