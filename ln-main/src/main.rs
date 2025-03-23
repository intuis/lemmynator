mod action;
mod app;
mod tui;
pub mod types;
mod ui;

use anyhow::Result;
use app::App;
use tracing::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let log_file = std::fs::File::create("/tmp/lemmynator.log").unwrap();
    tracing_subscriber::fmt().with_writer(log_file).init();

    info!("Lemmynator is starting");

    let mut app = App::new().await?;
    app.run().await?;

    info!("Lemmynator is quitting");

    Ok(())
}
