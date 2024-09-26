use erm::prelude::*;
use futures::TryStreamExt as _;

#[derive(Component, Debug)]
pub struct Name(String);

#[derive(Component, Debug)]
pub struct Age(u32);

#[tokio::main]
async fn main() {
    // Create an Sqlite backend using u64 as entity IDs
    let backend: SqliteBackend<i64> = SqliteBackend::in_memory().await;

    // This creates the component tables where data will be persisted.
    backend.register::<Name>().await.unwrap();
    backend.register::<Age>().await.unwrap();

    let jimothy = 1;

    backend
        .insert(&jimothy, &(Name("Jimothy".to_string()), Age(10)))
        .await;

    let andrea = 2;
    backend
        .insert(&andrea, &(Name("Andrea".to_string()), Age(32)))
        .await;

    #[derive(Archetype, Debug)]
    struct Person {
        name: Name,
        age: Age,
    }

    let people = backend
        .list_all::<Person>()
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
}
