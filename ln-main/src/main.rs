mod action;
mod app;
mod tui;
pub mod types;
mod ui;

use anyhow::Result;
use app::App;
use ln_config::Config;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let config = Config::init()?;

    let mut app = App::new(config).await?;
    app.run().await?;

    Ok(())
}
