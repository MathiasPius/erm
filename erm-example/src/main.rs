use erm::{Archetype, Component};
use futures::StreamExt as _;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Executor as _,
};

#[derive(Debug, Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Component)]
struct Label {
    label: String,
}

#[derive(Debug, Archetype)]
struct PhysicsObject {
    pub label: Label,
    pub position: Position,
}

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::new().in_memory(true);

    let db = SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await
        .unwrap();

    Position::create(&db).await.unwrap();
    Label::create(&db).await.unwrap();

    let to_insert = PhysicsObject {
        position: Position { x: 111.0, y: 222.0 },
        label: Label {
            label: "Something goes here?".to_string(),
        },
    };

    to_insert.insert(&db, &"c").await.unwrap();

    println!("{:#?}", PhysicsObject::get(&db, &"c").await.unwrap());

    while let Some(obj) = PhysicsObject::list(&db).next().await {
        println!("{obj:#?}");
    }
}
