use erm::prelude::*;
use futures::TryStreamExt as _;

#[derive(Component, Debug)]
struct Name(String);

#[derive(Component, Debug)]
struct Legs(i64);

#[derive(Component, Debug)]
struct Furniture;

#[derive(Component, Debug)]
struct Animal;

#[tokio::main]
async fn main() {
    // Create an Sqlite backend using u64 as entity IDs
    let backend: SqliteBackend<i64> = SqliteBackend::in_memory().await;

    // This creates the component tables where data will be persisted.
    backend.register::<Name>().await.unwrap();
    backend.register::<Legs>().await.unwrap();

    // And our marker components.
    backend.register::<Furniture>().await.unwrap();
    backend.register::<Animal>().await.unwrap();

    let elephant = 1;
    backend
        .insert(&elephant, &(Name("Elephant".to_string()), Legs(4), Animal))
        .await;

    let snake = 2;
    backend
        .insert(&snake, &(Name("Snake".to_string()), Animal))
        .await;

    let stool = 3;
    backend
        .insert(&stool, &(Name("Stool".to_string()), Legs(3), Furniture))
        .await;

    let table = 4;
    backend
        .insert(&table, &(Name("Table".to_string()), Legs(4), Furniture))
        .await;

    let legged_things = backend
        .list::<Name>()
        .with::<Legs>()
        .components()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("{legged_things:#?}");
    // [
    //     Name(
    //         "Elephant",
    //     ),
    //     Name(
    //         "Stool",
    //     ),
    //     Name(
    //         "Table",
    //     ),
    // ]

    let legless_animals = backend
        .list::<Name>()
        .with::<Animal>()
        .without::<Legs>()
        .components()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("{legless_animals:#?}");
    // [
    //     Name(
    //         "Snake",
    //     ),
    // ]
}
