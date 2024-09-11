use erm::{Archetype, Component};
use futures::StreamExt as _;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Executor as _,
};

#[derive(Debug, Component)]
struct Position {
    name: String,
    x: u32,
    y: u32,
}

#[derive(Debug, Component)]
struct Label {
    label: String,
    label2: String,
}

#[derive(Debug, Archetype)]
struct PhysicsObject {
    pub label: Label,
    pub position: Position,
}

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::new()
        //.filename("test.sqlite3")
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

    Position::create::<_, String>(&db).await.unwrap();
    Label::create::<_, String>(&db).await.unwrap();

    let to_insert = PhysicsObject {
        position: Position {
            name: "lol?".to_string(),
            x: 111,
            y: 222,
        },
        label: Label {
            label: "Something goes here?".to_string(),
            label2: "Label 2".to_string(),
        },
    };

    to_insert.insert(&db, &"a").await;
    to_insert.insert(&db, &"c").await;

    let mut stream = PhysicsObject::list::<String, _>(&db);
    while let Some(result) = stream.next().await {
        let (entity, obj) = result.unwrap();
        println!("{entity}: {obj:#?}");
    }

    //println!("{:#?}", PhysicsObject::get(&db, &"c").await.unwrap());
}
