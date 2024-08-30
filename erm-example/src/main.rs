use erm::{
    backend::{sqlite::SqliteBackend, Backend},
    entity::{GenerateUnique, Uuid},
};
use erm_derive::Component;
use sqlx::SqlitePool;

#[derive(Debug, Component)]
struct Position {
    x: f32,
    y: f32,
}

#[tokio::main]
async fn main() {
    let backend = SqliteBackend::new(SqlitePool::connect(":memory:").await.unwrap());

    let entity = Uuid::generate_unique();

    let position = Position { x: 1.0, y: 2.0 };

    backend.insert(entity, position).await;
}
