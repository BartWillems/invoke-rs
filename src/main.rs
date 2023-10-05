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
async fn main() {
    pretty_env_logger::init();

    let (client, mut receiver) = Client::connect("http://192.168.0.50:9090".into()).await;

    let bot = Bot::from_env();

    let responder_bot = bot.clone();

    let ai = client.clone();
    tokio::task::spawn(async move {
        let mut queue: HashMap<BatchId, (ChatId, MessageId)> = HashMap::new();

        let bot = responder_bot.clone();
        while let Some(update) = receiver.recv().await {
            log::info!("Update::{update:?}");

            handle_update(&bot, &ai, update, &mut queue)
                .await
                .map_err(|error| {
                    log::error!("failed to handle update: {error}");
                })
                .ok();
        }
    });

    telegram::handler(bot, client).dispatch().await;
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
            queue.insert(id, (chat_id, message_id));
        }
        Update::Progress { id: _ } => {}
        Update::Finished { id, image_url } => {
            let (chat_id, message_id) = queue.remove(&id).unwrap();

            let bytes = ai.download_image(image_url).await?;

            bot.send_photo(chat_id, InputFile::memory(bytes))
                .reply_to_message_id(message_id)
                .send()
                .await?;
        }
    }

    Ok(())
}
