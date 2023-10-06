use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

use crate::client::Client;
use crate::models::{Enqueue, ModelName};

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Generate a picture out of thin air and transistors")]
    AImg(String),
    #[command(description = "Generate an drawing out of thin air and transistors")]
    Draw(String),
}

pub fn handler(
    bot: Bot,
    ai: Client,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(
            |bot: Bot, ai: Client, msg: Message, command: Command| async move {
                log::debug!(
                    "Incomming command: `{command:?}`, Group ID: `{}`",
                    msg.chat.id
                );

                match command {
                    Command::Help => {
                        bot.send_message(msg.chat.id, Command::descriptions().to_string())
                            .reply_to_message_id(msg.id)
                            .send()
                            .await?;
                    }
                    Command::AImg(prompt) => {
                        log::info!("Generating image for prompt: `{prompt}`");

                        let res = ai.enqueue_text_to_image(prompt, msg.chat.id, msg.id).await;

                        match res {
                            Ok(enqueued) => log::info!("enqueued: {enqueued:?}"),
                            Err(error) => {
                                log::error!("Failed to enqueue draw-prompt: {error}");
                                bot.send_message(msg.chat.id, "Failed to generate image")
                                    .reply_to_message_id(msg.id)
                                    .send()
                                    .await?;
                            }
                        }
                    }

                    Command::Draw(prompt) => {
                        log::info!("Generating draw-image for prompt: `{prompt}`");

                        let enqueue = Enqueue::from_prompt(prompt)
                            .with_model(ModelName::ChildrensStoriesV1SemiReal);

                        let res = ai.enqueue_text_to_image(enqueue, msg.chat.id, msg.id).await;

                        match res {
                            Ok(enqueued) => log::info!("enqueued: {enqueued:?}"),
                            Err(error) => {
                                log::error!("Failed to enqueue draw-prompt: {error}");
                                bot.send_message(msg.chat.id, "Failed to generate image")
                                    .reply_to_message_id(msg.id)
                                    .send()
                                    .await?;
                            }
                        }
                    }
                }
                respond(())
            },
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ai])
        .enable_ctrlc_handler()
        .build()
}
