use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

use crate::client::Client;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Generate an image out of thin air")]
    AImg(String),
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
                            .await
                            .unwrap();
                    }
                    Command::AImg(prompt) => {
                        let res = ai
                            .enqueue_text_to_image(prompt, msg.chat.id, msg.id)
                            .await
                            .unwrap();

                        log::info!("enqueued: {res:?}");
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

pub async fn responder(bot: Bot, msg: Message, command: Command) -> ResponseResult<()> {
    log::debug!(
        "Incomming command: `{command:?}`, Group ID: `{}`",
        msg.chat.id
    );

    match command {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .reply_to_message_id(msg.id)
                .send()
                .await
                .unwrap();
        }
        Command::AImg(_query) => {
            bot.send_photo(msg.chat.id, InputFile::url(
                "http://192.168.0.50:9090/api/v1/images/i/5bac8aa2-b373-421a-86f2-43558a738cc7.png/full".try_into().unwrap(),
            )).reply_to_message_id(msg.id).send().await.unwrap();
        }
    }

    Ok(())
}
