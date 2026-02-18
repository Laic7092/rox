use anyhow::Result;

use brk::run_cli;

#[tokio::main]
async fn main() -> Result<()> {
    run_cli().await
}
