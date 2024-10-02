use sqlx::Database;

use crate::entity::EntityPrefixedQuery;

pub trait Removable<DB: Database>: Sized {
    fn remove<'query, EntityId>(query: &mut EntityPrefixedQuery<'query, DB, EntityId>)
    where
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
macro_rules! impl_remove_for_db{
    ($db:ty, $($list:ident:$index:tt),*) => {
        impl<$($list),*> Removable<$db> for ($($list,)*)
        where
            $($list: Removable<$db>,)*
        {
            fn remove<'query, EntityId>(
                query: &mut EntityPrefixedQuery<'query, $db, EntityId>,
            ) where
                EntityId: sqlx::Encode<'query, $db> + sqlx::Type<$db> + Clone + 'query,
            {
                $(
                    {
                        #[allow(unused)]
                        <$list as Removable<$db>>::remove(query);
                    }
                )*
            }
        }
    };
}

macro_rules! impl_compound {
    ($($list:ident:$index:tt),*) => {
        #[cfg(feature = "sqlite")]
        impl_remove_for_db!(sqlx::Sqlite, $($list:$index),*);
        #[cfg(feature = "postgres")]
        impl_remove_for_db!(sqlx::Postgres, $($list:$index),*);
        #[cfg(feature = "mysql")]
        impl_remove_for_db!(sqlx::MySql, $($list:$index),*);
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
