pub mod client;
pub mod models;
pub mod telegram;

use client::Client;

use models::BatchId;
use std::collections::HashMap;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::{
    requests::Requester,
    types::{ChatId, MessageId},
    Bot,
};

#[derive(Debug)]
pub enum Update {
    Started {
        id: BatchId,
        chat_id: ChatId,
        message_id: MessageId,
    },
    Progress {
        id: BatchId,
    },
    Finished {
        id: BatchId,
        image_url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let invoke_ai_url = std::env::var("INVOKE_AI_URL")?;
    log::info!("Trying to connect to InvokeAI on {invoke_ai_url}");
    let (client, mut receiver) = Client::connect(invoke_ai_url).await?;

    log::info!("Initialised InvokeAI client");
    let bot = Bot::from_env();

    log::info!("Initialised Telegram bot");
    let responder_bot = bot.clone();

    let ai = client.clone();
    tokio::task::spawn(async move {
        let mut queue: HashMap<BatchId, (ChatId, MessageId)> = HashMap::new();

        let bot = responder_bot.clone();
        while let Some(update) = receiver.recv().await {
            handle_update(&bot, &ai, update, &mut queue)
                .await
                .map_err(|error| {
                    log::error!("failed to handle update: {error}");
                })
                .ok();
        }

        log::info!("out of receiver loop, what the fuck");
    });

    log::info!("Ready to start handling messages...");
    telegram::handler(bot, client).dispatch().await;

    Ok(())
}

async fn handle_update(
    bot: &Bot,
    ai: &Client,
    update: Update,
    queue: &mut HashMap<BatchId, (ChatId, MessageId)>,
) -> Result<(), Box<dyn std::error::Error>> {
    match update {
        Update::Started {
            id,
            chat_id,
            message_id,
        } => {
            log::info!("started processing {id:?}");
            queue.insert(id, (chat_id, message_id));
        }
        Update::Progress { id } => {
            log::debug!("processing update {id:?}");
        }
        Update::Finished { id, image_url } => {
            log::info!("processing finished {id:?}, url: {image_url}");

            let (chat_id, message_id) = match queue.remove(&id) {
                Some((chat_id, message_id)) => (chat_id, message_id),
                None => {
                    log::debug!("Received image that's not in our queue, ignoring");
                    return Ok(());
                }
            };

            let bytes = ai.download_image(image_url).await?;

            bot.send_photo(chat_id, InputFile::memory(bytes))
                .reply_to_message_id(message_id)
                .send()
                .await?;
        }
    }

    Ok(())
}
