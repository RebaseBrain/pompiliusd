use std::{error::Error, future::pending};
use zbus::{connection, interface};

struct Greeter {
    count: u64
}

#[interface(name = "org.zbus.cloud_api")]
impl Greeter {
    // Can be `async` as well.
    fn say_hello(&mut self, name: &str) -> String {
        self.count += 1;
        format!("Hello {}! I have been called {} times.", name, self.count)
    }
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let greeter = Greeter { count: 0 };
    let _conn = connection::Builder::session()?
        .name("org.zbus.cloud_api")?
        .serve_at("/org/zbus/cloud_api", greeter)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
