//! The main binary entrypoint.

use rtzlib::base::types::Void;

#[tokio::main]
async fn main() -> Void {
    println!("Hello, world!");

    Ok(())
}


