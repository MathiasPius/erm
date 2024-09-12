use erm::{Archetype, Component};
use futures::StreamExt as _;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[derive(Debug, Component, PartialEq, Eq)]
struct Position {
    posname: String,
    x: u32,
    y: u32,
}

#[derive(Debug, Component, PartialEq, Eq)]
struct Label {
    label: String,
    label2: String,
}

#[derive(Debug, Archetype, PartialEq, Eq)]
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

    Position::create_component_table::<String>(&db)
        .await
        .unwrap();
    Label::create_component_table::<String>(&db).await.unwrap();

    let to_insert = PhysicsObject {
        position: Position {
            posname: "lol?".to_string(),
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

    let replacement = PhysicsObject {
        position: Position {
            posname: "lmao?".to_string(),
            x: 333,
            y: 444,
        },
        label: Label {
            label: "Something else here?".to_string(),
            label2: "Label 3".to_string(),
        },
    };

    replacement.update(&db, "a").await;

    let entity = "a".to_string();

    assert_eq!(replacement, PhysicsObject::get(&db, &entity).await.unwrap());

    let mut stream = PhysicsObject::list::<String>(&db);
    while let Some(result) = stream.next().await {
        let (entity, obj) = result.unwrap();
        println!("{entity}: {obj:#?}");
    }

    //println!("{:#?}", PhysicsObject::get(&db, &"c").await.unwrap());
}
