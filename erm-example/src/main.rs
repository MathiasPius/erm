use erm::prelude::*;
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

#[derive(Debug)]
struct MyWeirdThing(String);

impl From<String> for MyWeirdThing {
    fn from(value: String) -> Self {
        MyWeirdThing(value)
    }
}
impl Into<String> for &MyWeirdThing {
    fn into(self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Component)]
#[erm(table = "parents")]
struct Parent {
    #[erm(intermediate = String)]
    pub parent: MyWeirdThing,
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

    backend.register::<FriendlyName>().await.unwrap();
    backend.register::<Position>().await.unwrap();
    backend.register::<Parent>().await.unwrap();

    let _alice = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Alice".to_string(),
            },
            Position { x: 10, y: 20 },
        ))
        .await;

    let _bob = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Bob".to_string(),
            },
            Position { x: 30, y: 30 },
            Parent {
                parent: MyWeirdThing("Alice".to_string()),
            },
        ))
        .await;

    let charlie = backend
        .spawn(&(
            FriendlyName {
                friendly_name: "Charlie".to_string(),
            },
            Position { x: 40, y: 40 },
            Parent {
                parent: MyWeirdThing("Bob".to_string()),
            },
        ))
        .await;

    #[derive(Debug, Archetype)]
    #[allow(unused)]
    struct Person {
        name: FriendlyName,
        parent: Parent,
    }

    let children: Vec<_> = backend
        .list::<Person>()
        .and(Parent::FIELDS.parent.eq("Bob".to_string()))
        .fetch()
        .collect()
        .await;

    assert_eq!(children.len(), 1);
    println!("{children:#?}");

    backend.remove::<Person>(&charlie).await;

    let children: Vec<_> = backend
        .list::<Person>()
        .and(Parent::FIELDS.parent.eq("Bob".to_string()))
        .fetch()
        .collect()
        .await;

    assert!(children.is_empty());
}
