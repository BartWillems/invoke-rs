use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

use crate::client::Client;
use crate::models::Enqueue;

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
    #[command(description = "Generate a gigachad picture")]
    Gigachad(String),
    #[command(description = "Generate an anime drawing")]
    Anime(String),
    #[command(description = "LESGO LEGO")]
    Lego(String),
}

impl Command {
    fn override_prompt(&mut self, prompt: &'static str) {
        match self {
            Command::Help => {}
            Command::AImg(_) => *self = Command::AImg(prompt.to_string()),
            Command::Draw(_) => *self = Command::Draw(prompt.to_string()),
            Command::Gigachad(_) => *self = Command::Gigachad(prompt.to_string()),
            Command::Anime(_) => *self = Command::Anime(prompt.to_string()),
            Command::Lego(_) => *self = Command::Lego(prompt.to_string()),
        }
    }
}

pub fn handler(
    bot: Bot,
    ai: Client,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(
            |bot: Bot, ai: Client, msg: Message, mut command: Command| async move {
                log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);

                let enqueue = match command {
                    Command::Help => {
                        bot.send_message(msg.chat.id, Command::descriptions().to_string())
                            .reply_to_message_id(msg.id)
                            .send()
                            .await?;

                        return Ok(());
                    }
                    Command::AImg(prompt) => Enqueue::from_prompt(prompt),
                    Command::Draw(prompt) => Enqueue::from_prompt(prompt).drawing(),
                    Command::Gigachad(prompt) => Enqueue::from_prompt(prompt).gigachad(),
                    Command::Anime(prompt) => Enqueue::from_prompt(prompt).anime(),
                    Command::Lego(prompt) => Enqueue::from_prompt(prompt).lego(),
                };

                let res = ai.enqueue_text_to_image(enqueue, msg.chat.id, msg.id).await;

                match res {
                    Ok(enqueued) => log::info!("enqueued: {enqueued:?}"),
                    Err(error) => {
                        log::error!("Failed to enqueue generate image: {error}");
                        bot.send_message(msg.chat.id, "Failed to generate image")
                            .reply_to_message_id(msg.id)
                            .send()
                            .await?;
                    }
                }

                Ok(())
            },
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ai])
        .default_handler(|_| async {})
        .build()
}
