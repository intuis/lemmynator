mod action;
mod app;
mod tui;
mod ui;

use anyhow::Result;
use app::App;
use ln_config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::init()?;

    let mut app = App::new(config).await?;
    app.run().await?;

    Ok(())
}
