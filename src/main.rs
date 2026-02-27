mod app;

use anyhow::Result;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

fn main() -> Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .without_time()
        .init();

    app::run()
}
