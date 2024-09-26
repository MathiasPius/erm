use erm::prelude::{Component, SqliteBackend};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[derive(Component)]
pub struct Name(String);

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

    // Create an Sqlite backend using u64 as entity IDs
    let backend: SqliteBackend<u64> = SqliteBackend::new(db);
}
