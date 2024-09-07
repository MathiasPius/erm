use erm::{Archetype, Component};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

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

    let result = PhysicsObject::get(&db, 1234).await.unwrap();

    /*
    backend.init::<Position>().await;
    backend.init::<Velocity>().await;

    let entity = Uuid::generate_unique();

    backend.insert(&entity, Position { x: 1.0, y: 2.0 }).await;
    backend.insert(&entity, Velocity { x: 8.0, y: 9.0 }).await;

    let physics_object: PhysicsObject = backend.get(entity).await.unwrap();

    println!("{physics_object:#?}");

    for object in backend.list::<PhysicsObject>().await {
        println!("{object:#?}");
    }
     */
}
