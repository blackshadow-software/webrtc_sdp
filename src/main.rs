use client::run_client;
pub mod broad_cast;
pub mod client;
pub mod screen_capture;

#[tokio::main]
async fn main() {
    _ = run_client().await;
}
