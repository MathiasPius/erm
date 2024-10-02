use erm::prelude::*;
use futures::TryStreamExt as _;

#[derive(Component, Debug, PartialEq, Eq)]
pub struct OrderId(String);

#[derive(Component, Debug, PartialEq)]
pub struct Payment {
    account_id: String,
}

#[derive(Component, Debug, PartialEq)]
pub struct ShippedTo {
    address: String,
}

#[tokio::main]
async fn main() {
    // Create an Sqlite backend using u64 as entity IDs
    let backend: SqliteBackend<i64> = SqliteBackend::in_memory().await;

    // This creates the component tables where data will be persisted.
    backend.register::<OrderId>().await.unwrap();
    backend.register::<Payment>().await.unwrap();
    backend.register::<ShippedTo>().await.unwrap();

    // Outstanding order, which has been neither paid nor shipped..
    backend.insert(&1, &OrderId("#1234".to_string())).await;

    // This one has been paid, but not yet shipped.
    backend
        .insert(
            &2,
            &(
                OrderId("#999".to_string()),
                Payment {
                    account_id: "1234 56789123".to_string(),
                },
            ),
        )
        .await;

    // This order has been ordered, paid and shipped.
    backend
        .insert(
            &3,
            &(
                OrderId("#10000020".to_string()),
                Payment {
                    account_id: "9876 12345678".to_string(),
                },
                ShippedTo {
                    address: "123 West Westington Street, 999 Capital".to_string(),
                },
            ),
        )
        .await;

    // List all items that have been shipped
    let shipped_items = backend
        .list::<OrderId>()
        .with::<ShippedTo>()
        .components()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("{shipped_items:#?}");
    // [
    //     OrderId(
    //         "#10000020",
    //     ),
    // ]

    assert_eq!(shipped_items[0], OrderId("#10000020".to_string()));

    // List all items which have not yet been paid for.
    let unpaid_items = backend
        .list::<OrderId>()
        .without::<Payment>()
        .components()
        .fetch()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("{unpaid_items:#?}");
    // [
    //      OrderId(
    //          "#1234",
    //     ),
    // ]

    assert_eq!(unpaid_items[0], OrderId("#1234".to_string()));
}
