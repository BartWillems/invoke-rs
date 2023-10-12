use teloxide::prelude::Update as TelegramUpdate;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::{
    payloads::SendMessageSetters,
    requests::{Request as RequestExt, Requester},
    types::UserId,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::handler::Update;
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
    sender: UnboundedSender<Update>,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let handler = TelegramUpdate::filter_message()
        .filter_command::<Command>()
        .endpoint(
            |bot: Bot, sender: UnboundedSender<Update>, msg: Message, command: Command| async move {
                log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);

                let user = match msg.from() {
                    Some(user) => user,
                    None => {
                        log::warn!("Received a command without a user");
                        return Ok(());
                    }
                };

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

                sender
                    .send(Update::Requested {
                        enqueue,
                        chat_id: msg.chat.id,
                        user_id: user.id,
                        message_id: msg.id,
                    })
                    .expect("failed to send update, this is bad");

                Ok(())
            },
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![sender])
        .default_handler(|_| async {})
        .build()
}
