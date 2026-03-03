use anyhow::Result;

use rox::run_cli;

#[tokio::main]
async fn main() -> Result<()> {
    run_cli().await
}
