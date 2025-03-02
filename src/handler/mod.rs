use teloxide::Bot;

pub mod invoke;
pub mod local;
pub mod ollama;
pub mod store;

// pub use store::Store;

use crate::local_ai::Prompts;
use crate::utils::languages::LanguageDetector;
use crate::utils::SearXng;
use crate::AppConfig;

pub struct Handler;

impl Handler {
    pub async fn dispatch(config: AppConfig) -> anyhow::Result<()> {
        let AppConfig {
            invoke_ai_url,
            local_ai_url,
            teloxide_token,
            ollama_url,
            telegram_admin_user_id,
            max_in_progress,
            sqlite_path,
            enable_french_detection,
            ollama_model,
            searxng_url,
            fact_check_path,
        } = config;

        let bot = Bot::new(teloxide_token);

        let http_client = http_client();

        let invoke = invoke::Handler::new(
            invoke::Config {
                invoke_ai_url,
                max_in_progress,
            },
            bot.clone(),
            http_client.clone(),
        )
        .await;

        let prompts = Prompts::default();

        let local = local::Handler::try_new(
            local::Config {
                local_ai_url,
                max_in_progress,
            },
            bot.clone(),
            http_client.clone(),
            prompts.clone(),
        )?;

        let ollama = ollama::Handler::try_new(
            ollama::Config {
                api_uri: ollama_url,
                max_in_progress,
                model: ollama_model.clone(),
            },
            bot.clone(),
            http_client.clone(),
        )?;

        let store = crate::store::Store::new(&sqlite_path, bot.clone(), ollama_model).await?;

        let searxng = SearXng::new(http_client.clone(), searxng_url);

        let fact_check_engine = crate::telegram::fact_check::Engine::new(fact_check_path).await?;

        let mut telegram = crate::telegram::handler(crate::telegram::Context {
            cfg: crate::telegram::Config {
                admin_id: telegram_admin_user_id,
            },
            bot,
            store,
            invoke_notifier: invoke.notifier(),
            local_notifier: local.notifier(),
            ollama_notifier: ollama.notifier(),
            language: LanguageDetector::new(enable_french_detection),
            prompts,
            http_client,
            searxng,
            fact_check_engine,
        });

        log::info!("Starting all handlers...");
        futures::future::join4(
            invoke.start(),
            local.start(),
            ollama.start(),
            telegram.dispatch(),
        )
        .await;

        Ok(())
    }
}

fn http_client() -> reqwest::Client {
    use reqwest::header::{
        self, HeaderValue, ACCEPT, ACCEPT_ENCODING, UPGRADE_INSECURE_REQUESTS, USER_AGENT,
    };
    const USER_AGENT_VALUE: &str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0";
    const ACCEPT_VALUE: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9";
    const ACCEPT_ENCODING_VALUE: &str = "gzip, deflate, br";

    let mut headers = header::HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(ACCEPT, HeaderValue::from_static(ACCEPT_VALUE));
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static(ACCEPT_ENCODING_VALUE),
    );
    headers.insert(UPGRADE_INSECURE_REQUESTS, HeaderValue::from_static("1"));

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("failed to build http client")
}
