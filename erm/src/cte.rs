use std::marker::PhantomData;
use std::{collections::BTreeSet, fmt::Write};

use std::fmt::Result;

use sqlx::Database;

use crate::prelude::Deserializeable;

pub trait CommonTableExpression: std::fmt::Debug {
    fn table_name(&self, f: &mut dyn Write) -> Result;
    fn columns(&self, f: &mut dyn Write) -> Result;
    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>];
    fn serialize(&self, f: &mut dyn Write) -> Result;
    fn optional(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct Extract {
    pub table: &'static str,
    pub columns: &'static [&'static str],
}

impl CommonTableExpression for Extract {
    fn table_name(&self, f: &mut dyn Write) -> Result {
        write!(f, "{}", self.table)
    }

    fn columns(&self, f: &mut dyn Write) -> Result {
        for column in self.columns {
            write!(f, ",\n      __cte_{}__{}", self.table, column)?
        }

        Ok(())
    }

    fn serialize(&self, f: &mut dyn Write) -> Result {
        write!(
            f,
            "    select\n      entity as __cte_{table}__entity",
            table = self.table
        )?;
        for column in self.columns {
            write!(
                f,
                ",\n      {column} as __cte_{table}__{column}",
                table = self.table,
                column = column
            )?
        }
        write!(f, "\n    from\n      ")?;
        self.table_name(f)
    }

    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>] {
        &[]
    }
}

#[derive(Debug)]
pub struct Optional {
    pub inner: Box<dyn CommonTableExpression>,
}

impl CommonTableExpression for Optional {
    fn table_name(&self, f: &mut dyn Write) -> Result {
        self.inner.table_name(f)
    }

    fn columns(&self, f: &mut dyn Write) -> Result {
        self.inner.columns(f)
    }

    fn serialize(&self, f: &mut dyn Write) -> Result {
        self.inner.serialize(f)
    }

    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>] {
        self.inner.dependencies()
    }

    fn optional(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Merge {
    pub tables: Vec<Box<dyn CommonTableExpression>>,
}

impl CommonTableExpression for Merge {
    fn table_name(&self, f: &mut dyn Write) -> Result {
        let mut tables = self.tables.iter();
        let first = tables.next().unwrap();
        first.table_name(f)?;

        for table in tables {
            write!(f, "_")?;
            table.table_name(f)?;
        }

        Ok(())
    }

    fn columns(&self, f: &mut dyn Write) -> Result {
        for inner in &self.tables {
            inner.columns(f)?;
        }

        Ok(())
    }

    fn serialize(&self, f: &mut dyn Write) -> Result {
        let mut tables = self.tables.iter();
        let first = tables.next().unwrap();

        write!(f, "{}", "    select\n      __cte_")?;
        first.table_name(f)?;
        write!(f, "__entity")?;
        self.columns(f)?;
        write!(f, "{}", "\n    from\n      __cte_")?;
        first.table_name(f)?;

        for table in tables {
            if table.optional() {
                write!(f, "\n    left join\n      __cte_")?;
            } else {
                write!(f, "\n    inner join\n      __cte_")?;
            }
            table.table_name(f)?;
            write!(f, "\n    on\n      __cte_")?;
            first.table_name(f)?;
            write!(f, "__entity = __cte_")?;
            table.table_name(f)?;
            write!(f, "__entity")?;
        }

        Ok(())
    }

    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>] {
        &self.tables
    }
}

#[derive(Debug)]
pub struct Include {
    pub inner: [Box<dyn CommonTableExpression>; 2],
}

impl CommonTableExpression for Include {
    fn table_name(&self, f: &mut dyn Write) -> Result {
        self.inner[0].table_name(f)?;
        write!(f, "_including_")?;
        self.inner[1].table_name(f)
    }

    fn columns(&self, f: &mut dyn Write) -> Result {
        self.inner[0].columns(f)
    }

    fn serialize(&self, f: &mut dyn Write) -> Result {
        write!(f, "    select\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "__entity")?;
        self.columns(f)?;
        write!(f, "\n    from\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "\n    inner join\n      __cte_")?;
        self.inner[1].table_name(f)?;
        write!(f, "\n    on\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "__entity = __cte_")?;
        self.inner[1].table_name(f)?;
        write!(f, "__entity")
    }

    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>] {
        &self.inner
    }
}

#[derive(Debug)]
pub struct Exclude {
    pub inner: [Box<dyn CommonTableExpression>; 2],
}

impl CommonTableExpression for Exclude {
    fn table_name(&self, f: &mut dyn Write) -> Result {
        self.inner[0].table_name(f)?;
        write!(f, "_excluding_")?;
        self.inner[1].table_name(f)
    }

    fn columns(&self, f: &mut dyn Write) -> Result {
        self.inner[0].columns(f)
    }

    fn serialize(&self, f: &mut dyn Write) -> Result {
        write!(f, "{}", "    select\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "__entity")?;
        self.columns(f)?;
        write!(f, "\n    from\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "\n    left join\n      __cte_")?;
        self.inner[1].table_name(f)?;
        write!(f, "\n    on\n      __cte_")?;
        self.inner[0].table_name(f)?;
        write!(f, "__entity = __cte_")?;
        self.inner[1].table_name(f)?;
        write!(f, "__entity\n    where __cte_")?;
        self.inner[1].table_name(f)?;
        write!(f, "__entity is null")
    }

    fn dependencies(&self) -> &[Box<dyn CommonTableExpression>] {
        &self.inner
    }
}

pub(crate) fn serialize(
    cte: &dyn CommonTableExpression,
) -> ::core::result::Result<String, std::fmt::Error> {
    let mut ctes = BTreeSet::new();

    #[derive(Eq, Ord)]
    struct SerializedExpression {
        name: String,
        contents: String,
    }

    impl PartialEq for SerializedExpression {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
        }
    }

    impl PartialOrd for SerializedExpression {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.name.partial_cmp(&other.name)
        }
    }

    fn serialize_into(
        cte: &dyn CommonTableExpression,
        ctes: &mut BTreeSet<SerializedExpression>,
    ) -> Result {
        let mut serialized = SerializedExpression {
            name: String::new(),
            contents: String::new(),
        };

        cte.table_name(&mut serialized.name)?;

        if !ctes.contains(&serialized) {
            for dependency in cte.dependencies() {
                serialize_into(dependency.as_ref(), ctes)?;
            }

            write!(
                serialized.contents,
                "  __cte_{table_name} as (\n",
                table_name = serialized.name
            )
            .unwrap();

            cte.serialize(&mut serialized.contents)?;
            serialized.contents.push_str("\n  )");
            ctes.insert(serialized);
        }
        Ok(())
    }

    serialize_into(cte, &mut ctes)?;

    let mut statement = String::from("with\n");
    for (index, serialized_cte) in ctes.into_iter().enumerate() {
        if index != 0 {
            statement.push_str(",\n");
        }
        statement.push_str(&serialized_cte.contents);
    }

    statement.push_str("\nselect * from __cte_");
    cte.table_name(&mut statement)?;
    statement.push_str("\n");

    Ok(statement)
}

pub trait Filter<DB: Database> {
    fn cte(cte: Box<dyn CommonTableExpression>) -> Box<dyn CommonTableExpression>;
}

// This is for the empty unfiltered case.
impl<DB: Database> Filter<DB> for () {
    fn cte(cte: Box<dyn CommonTableExpression>) -> Box<dyn CommonTableExpression> {
        cte
    }
}

pub struct With<T>(PhantomData<T>);

impl<T, DB: Database> Filter<DB> for With<T>
where
    T: Deserializeable<DB>,
{
    fn cte(cte: Box<dyn CommonTableExpression>) -> Box<dyn CommonTableExpression> {
        Box::new(Include {
            inner: [cte, <T as Deserializeable<DB>>::cte()],
        })
    }
}
pub struct Without<T>(PhantomData<T>);

impl<T, DB: Database> Filter<DB> for Without<T>
where
    T: Deserializeable<DB>,
{
    fn cte(cte: Box<dyn CommonTableExpression>) -> Box<dyn CommonTableExpression> {
        Box::new(Exclude {
            inner: [cte, <T as Deserializeable<DB>>::cte()],
        })
    }
}

macro_rules! impl_filter_for_tuple{
    ($($list:ident),*) => {
        impl<DB, $($list),*> Filter<DB> for ($($list,)*)
        where
            DB: Database,
            $($list: Filter<DB>,)*
        {
            fn cte(cte: Box<dyn CommonTableExpression>) -> Box<dyn CommonTableExpression> {
                $(let cte = <$list as Filter<DB>>::cte(cte);)*
                cte
            }
        }
    }
}

impl_filter_for_tuple!(T1);
impl_filter_for_tuple!(T1, T2);
impl_filter_for_tuple!(T1, T2, T3);
impl_filter_for_tuple!(T1, T2, T3, T4);
impl_filter_for_tuple!(T1, T2, T3, T4, T5);
impl_filter_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_filter_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_filter_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);

#[test]
fn test_build() {
    let positions = Extract {
        table: "positions",
        columns: &["x", "y"],
    };

    let names = Extract {
        table: "named",
        columns: &["first", "last"],
    };

    let merge = Merge {
        tables: vec![Box::new(positions), Box::new(names)],
    };

    let exclude = Exclude {
        inner: [
            Box::new(merge),
            Box::new(Extract {
                table: "address",
                columns: &[],
            }),
        ],
    };

    let parents = Extract {
        table: "parents",
        columns: &[],
    };

    let include = Include {
        inner: [Box::new(exclude), Box::new(parents)],
    };

    println!("{}", serialize(&include).unwrap());
}
