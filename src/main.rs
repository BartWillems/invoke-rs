use config::Config;

pub mod handler;
pub mod invoke_ai;
pub mod local_ai;
pub mod telegram;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init();

    log::info!("Loading config...");
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("app").try_parsing(true))
        .build()?;

    let config: handler::Config = config.try_deserialize()?;

    log::info!("Initializing...");
    handler::Handler::dispatch(config).await?;

    Ok(())
}
