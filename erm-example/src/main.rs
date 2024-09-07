use erm::{Archetype, Component};
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

    db.execute(
        r#"
            create table if not exists position(
                entity text primary key,
                x real,
                y real
            );
            "#,
    )
    .await
    .unwrap();

    db.execute(
        r#"
            create table if not exists label(
                entity text primary key,
                label text
            );
            "#,
    )
    .await
    .unwrap();

    db.execute(
        r#"
            insert or ignore into position(entity, x, y) values('a', 10.0, 20.0);
            insert or ignore into position(entity, x, y) values('b', 30.0, 40.0);
            insert or ignore into label(entity, label) values("a", "first");
            insert or ignore into label(entity, label) values("b", "second");
        "#,
    )
    .await
    .unwrap();

    let result = PhysicsObject::get(&db, &"a").await.unwrap();

    println!("{result:#?}");

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
