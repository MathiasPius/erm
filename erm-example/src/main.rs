use erm::{
    backend::{Backend, SqliteBackend},
    Archetype, Component,
};
use futures::StreamExt as _;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uuid::Uuid;

#[derive(Debug, Component, PartialEq, Eq)]
struct FriendlyName {
    friendly_name: String,
}

#[derive(Debug, Component, PartialEq, Eq)]
struct Position {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Component, PartialEq, Eq)]
struct Parent {
    pub parent: Uuid,
}

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await
        .unwrap();

    let backend: SqliteBackend<Uuid> = SqliteBackend::new(db);

    backend
        .init::<(FriendlyName, Position, Parent)>()
        .await
        .unwrap();

    let alice = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Alice".to_string(),
            },
            Position { x: 10, y: 20 },
        ))
        .await;

    let bob = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Bob".to_string(),
            },
            Position { x: 30, y: 30 },
            Parent { parent: alice },
        ))
        .await;

    let charlie = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Charlie".to_string(),
            },
            Position { x: 40, y: 40 },
            Parent { parent: bob },
        ))
        .await;

    #[derive(Debug, Archetype)]
    pub struct Person {
        name: FriendlyName,
        parent: Parent,
    }

    use erm::Reflect;
    let children: Vec<_> = Box::pin(backend.list::<Person, _>(Parent::FIELDS.parent.eq(bob)))
        .collect()
        .await;

    assert_eq!(children.len(), 1);
    println!("{children:#?}");

    backend.delete::<Person>(&charlie).await;

    let children: Vec<_> = Box::pin(backend.list::<Person, _>(Parent::FIELDS.parent.eq(bob)))
        .collect()
        .await;

    assert!(children.is_empty());
}
