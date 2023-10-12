pub mod handler;
pub mod invoke_ai;
pub mod models;
pub mod telegram;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let invoke_ai_url = std::env::var("INVOKE_AI_URL")?;

    log::info!("Initializing...");
    handler::Handler::try_new(invoke_ai_url)
        .await?
        .dispatch()
        .await;

    Ok(())
}
