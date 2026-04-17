use clouds_api::{Cloud, rclone_api::RcClone};
use reqwest::Client;
use std::{error::Error, future::pending};
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cloud = Cloud {
        rclone: RcClone {
            client: Client::new(),
            url: String::from("http://127.0.0.1:5572/"),
        },
    };
    let _conn = connection::Builder::session()?
        .name("org.zbus.pompiliusd")?
        .serve_at("/org/zbus/pompiliusd", cloud)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
