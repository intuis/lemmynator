mod action;
mod app;
mod tui;
pub mod types;
mod ui;

use anyhow::Result;
use app::App;
use ln_config::Config;
use tracing::{error, info, Level};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let log_file = std::fs::File::create("/tmp/lemmynator.log").unwrap();
    tracing_subscriber::fmt().with_writer(log_file).init();

    info!("Lemmynator is starting");

    let config = Config::init()?;

    let mut app = App::new(config).await?;
    app.run().await?;

    info!("Lemmynator is quitting");

    Ok(())
}
