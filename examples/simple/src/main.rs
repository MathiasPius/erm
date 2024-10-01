use erm::prelude::*;
use futures::TryStreamExt as _;

#[derive(Component, Debug)]
pub struct Name(String);

#[derive(Component, Debug, PartialEq)]
pub struct Age(i64);

#[tokio::main]
async fn main() {
    // Create an Sqlite backend using u64 as entity IDs
    let backend: SqliteBackend<i64> = SqliteBackend::in_memory().await;

    // This creates the component tables where data will be persisted.
    backend.register::<Name>().await.unwrap();
    backend.register::<Age>().await.unwrap();

    // Create our entities: Jimothy and Andrea
    //
    // Since we're just using i64s as our "Entity", our entities
    // are actually just numbers.
    let jimothy = 1;
    backend
        .insert(&jimothy, &(Name("Jimothy".to_string()), Age(10)))
        .await;

    let andrea = 2;
    backend
        .insert(&andrea, &(Name("Andrea".to_string()), Age(32)))
        .await;

    // Let's name an Archetype instead of just relying on a tuple.
    #[derive(Archetype, Debug)]
    #[allow(unused)]
    struct Person {
        name: Name,
        age: Age,
    }

    // List all the people we know
    let people = backend
        .list::<Person>()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("{people:#?}");
    // [
    //     (
    //         1,
    //         Person {
    //             name: Name(
    //                 "Jimothy",
    //             ),
    //             age: Age(
    //                 10,
    //             ),
    //         },
    //     ),
    //     (
    //         2,
    //         Person {
    //             name: Name(
    //                 "Andrea",
    //             ),
    //             age: Age(
    //                 32,
    //             ),
    //         },
    //     ),
    // ]

    backend.remove::<Person>(&jimothy).await;

    let remaining_names = backend
        .list::<Name>()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    // Check that only Andrea is left.
    assert_eq!(remaining_names.len(), 1);
    println!("{:#?}", remaining_names[0].1);
    // Name(
    //     "Andrea",
    // )

    // Fetch Andrea's age
    assert_eq!(backend.get::<Age>(&andrea).await.unwrap(), Age(32));
}
