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

    let third = PhysicsObject {
        position: Position { x: 111.0, y: 222.0 },
        label: Label {
            label: "Something goes here?".to_string(),
        },
    };

    third.insert(&db, &"c").await.unwrap();

    println!("{result:#?}");
}
