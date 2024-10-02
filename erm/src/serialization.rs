use sqlx::{query::Query, ColumnIndex, Database};

use crate::{cte::*, entity::EntityPrefixedQuery, row::OffsetRow, tables::Removable};

pub trait Deserializeable<DB: Database>: Sized {
    fn cte() -> Box<dyn CommonTableExpression>;
    fn deserialize(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;
}

pub trait Serializable<DB: Database>: Sized {
    fn serialize<'query>(
        &'query self,
        query: Query<'query, DB, <DB as Database>::Arguments<'query>>,
    ) -> Query<'query, DB, <DB as Database>::Arguments<'query>>;

    fn insert<'query, EntityId>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, EntityId>,
    ) where
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;

    fn update<'query, EntityId>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, EntityId>,
    ) where
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;
}

impl<T: Deserializeable<DB>, DB: Database> Deserializeable<DB> for Option<T>
where
    usize: ColumnIndex<<DB as Database>::Row>,
{
    fn cte() -> Box<dyn CommonTableExpression> {
        Box::new(Optional {
            inner: <T as Deserializeable<DB>>::cte(),
        })
    }

    fn deserialize(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error> {
        if row.is_null() {
            row.skip(1);
            Ok(None)
        } else {
            <T as Deserializeable<DB>>::deserialize(row).map(Some)
        }
    }
}

impl<T: Removable<DB>, DB: Database> Removable<DB> for Option<T> {
    fn remove<'query, EntityId>(query: &mut EntityPrefixedQuery<'query, DB, EntityId>)
    where
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        <T as Removable<DB>>::remove(query);
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
macro_rules! impl_deser_for_db{
    ($db:ty, $($list:ident:$index:tt),*) => {
        impl<$($list),*> Deserializeable<$db> for ($($list,)*)
        where
            $($list: Deserializeable<$db>,)*
        {
            fn cte() -> Box<dyn CommonTableExpression> {
                Box::new(Merge {
                    tables: vec![
                        $(<$list as Deserializeable<$db>>::cte(),)*
                    ],
                })
            }

            fn deserialize(
                row: &mut OffsetRow<<$db as Database>::Row>,
            ) -> Result<Self, sqlx::Error> {
                Ok((
                    $(
                        <$list as Deserializeable<$db>>::deserialize(row)?,
                    )*
                ))
            }
        }

        impl<$($list),*> Serializable<$db> for ($($list,)*)
        where
            $($list: Serializable<$db>,)*
        {
            fn serialize<'q>(
                &'q self,
                query: Query<'q, $db, <$db as Database>::Arguments<'q>>,
            ) -> Query<'q, $db, <$db as Database>::Arguments<'q>> {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    let query = self.$index.serialize(query);
                )*

                query
            }

            fn insert<'query, EntityId>(
                &'query self,
                query: &mut EntityPrefixedQuery<'query, $db, EntityId>
            )
            where
                EntityId: sqlx::Encode<'query, $db> + sqlx::Type<$db> + Clone + 'query
            {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    self.$index.insert(query);
                )*
            }

            fn update<'query, EntityId>(
                &'query self,
                query: &mut EntityPrefixedQuery<'query, $db, EntityId>
            )
            where
                EntityId: sqlx::Encode<'query, $db> + sqlx::Type<$db> + Clone + 'query
            {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    self.$index.update(query);
                )*
            }
        }
    }
}

macro_rules! impl_compound {
    ($($list:ident:$index:tt),*) => {
        #[cfg(feature = "sqlite")]
        impl_deser_for_db!(sqlx::Sqlite, $($list:$index),*);
        #[cfg(feature = "postgres")]
        impl_deser_for_db!(sqlx::Postgres, $($list:$index),*);
        #[cfg(feature = "mysql")]
        impl_deser_for_db!(sqlx::MySql, $($list:$index),*);
    };
}

impl_compound!(A:0, B:1);
impl_compound!(A:0, B:1, C:2);
impl_compound!(A:0, B:1, C:2, D:3);
impl_compound!(A:0, B:1, C:2, D:3, E:4);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
