use erm::{
    backend::{sqlite::SqliteBackend, Backend},
    entity::{GenerateUnique, Uuid},
};
use erm_derive::{Archetype, Component};
use sqlx::SqlitePool;

#[derive(Debug, Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Component)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, Archetype)]
struct PhysicsObject {
    pub position: Position,
    pub velocity: Velocity,
}

#[tokio::main]
async fn main() {
    let backend = SqliteBackend::new(SqlitePool::connect(":memory:").await.unwrap());

    backend.init::<Position>().await;
    backend.init::<Velocity>().await;

    let entity = Uuid::generate_unique();

    backend.insert(entity, Position { x: 1.0, y: 2.0 }).await;
    backend.insert(entity, Velocity { x: 8.0, y: 9.0 }).await;

    let physics_object: PhysicsObject = backend.get(entity).await.unwrap();

    println!("{physics_object:#?}");

    for object in backend.list::<PhysicsObject>().await {
        println!("{object:#?}");
    }
}
