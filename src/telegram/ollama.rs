use teloxide::{prelude::*, utils::command::BotCommands};
use url::Url;

use crate::{
    handler::ollama::{Identifier, Update},
    ollama,
};

use super::Context;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    Hey(String),
    Oi(String),
    Tldr,
    Summary(String),
    DeepSearch(String),
}

impl Command {
    fn override_prompt(&mut self, prompt: impl ToString) {
        match self {
            Command::Hey(_) => *self = Command::Hey(prompt.to_string()),
            Command::Oi(_) => *self = Command::Oi(prompt.to_string()),
            Command::Tldr => (), // todo maybe
            Command::Summary(_) => (),
            Command::DeepSearch(_) => (),
        }
    }

    fn misses_prompt(&self) -> bool {
        match self {
            Command::Tldr => false,
            Command::Hey(prompt)
            | Command::Oi(prompt)
            | Command::Summary(prompt)
            | Command::DeepSearch(prompt) => prompt.is_empty(),
        }
    }
}

pub async fn handler(
    ctx: Context,
    msg: Message,
    mut command: Command,
    overrides: super::admin::Overrides,
) -> Result<(), teloxide::RequestError> {
    log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);

    let user = match msg.from() {
        Some(user) => user,
        None => {
            log::warn!("Received a command without a user");
            return Ok(());
        }
    };

    if command.misses_prompt() {
        ctx.quick_reply(&msg, "You need to provide a valid prompt")
            .await;
        return Ok(());
    }

    if let Some(prompt) = overrides.get_override(user.id).await {
        command.override_prompt(prompt);
    }

    if user.id == UserId(172179034) {
        command.override_prompt("What are the hazards of driving on a flat tire?");
    }

    match command {
        Command::Hey(prompt) | Command::Oi(prompt) => {
            ctx.ollama_notifier.notify(Update::Requested {
                identifier: Identifier {
                    chat_id: msg.chat.id,
                    user_id: user.id,
                    message_id: msg.id,
                },
                prompt,
            })
        }
        Command::Tldr => {
            let chat_history = match ctx.store.chat_history(msg.chat.id).await {
                Ok(Some(history)) => history,
                Ok(None) => {
                    ctx.bot.send_message(msg.chat.id, "No chat content found. Please let me learn longer or adjust my permissions.")
                        .await?;
                    return Ok(());
                }
                Err(err) => {
                    log::error!("failed to fetch chat history: {err}");
                    ctx.bot.send_message(msg.chat.id, "failure").await?;
                    return Ok(());
                }
            };

            ctx.ollama_notifier.notify(Update::Requested {
                identifier: Identifier {
                    chat_id: msg.chat.id,
                    user_id: user.id,
                    message_id: msg.id,
                },
                prompt: chat_history,
            })
        }
        Command::Summary(text) => {
            let url = match Url::parse(&text) {
                Ok(url) => url,
                Err(_) if text.is_empty() => match find_url_in_reply(&msg) {
                    Some(url) => url,
                    None => {
                        ctx.quick_reply(&msg, "please provide the URL of a page to summarize")
                            .await;
                        return Ok(());
                    }
                },
                Err(_) => {
                    ctx.quick_reply(&msg, "Not a valid URL").await;
                    return Ok(());
                }
            };

            let is_internal_ip = url
                .host()
                .map(|host| match host {
                    url::Host::Ipv6(_) => false,
                    url::Host::Ipv4(ipv4) => ipv4.is_private(),
                    url::Host::Domain(domain) => domain.to_lowercase().as_str() == "localhost",
                })
                .unwrap_or_default();

            if is_internal_ip {
                ctx.quick_reply(&msg, "don't hack me hé klet").await;
                return Ok(());
            }

            let chat_id = msg.chat.id;
            let message_id = msg.id;
            let user_id = user.id;

            tokio::task::spawn(async move {
                let website_content = ctx
                    .http_client
                    .get(url.clone())
                    .send()
                    .await
                    .inspect_err(|error| log::error!("failed to call `{url}`, error: `{error}`"))?
                    .text()
                    .await
                    .inspect_err(|error| {
                        log::error!(
                            "failed to fetch full text response from `{url}`, error: `{error}`"
                        )
                    })?;

                let normalised = match readability::extractor::extract(&website_content, &url) {
                    Ok(product) => product,
                    Err(err) => {
                        log::error!("failed to extract document: {err}");
                        return Ok(());
                    }
                };

                ctx.ollama_notifier.notify(Update::Requested {
                    identifier: Identifier {
                        chat_id,
                        user_id,
                        message_id,
                    },
                    prompt: ollama::prompts::summary(normalised.text),
                });

                Result::<(), reqwest::Error>::Ok(())
            });
        }

        Command::DeepSearch(query) => {
            let chat_id = msg.chat.id;
            let message_id = msg.id;
            let user_id = user.id;

            tokio::task::spawn(async move {
                let results = ctx.searxng.search(&query).await?;

                ctx.ollama_notifier.notify(Update::Requested {
                    identifier: Identifier {
                        chat_id,
                        user_id,
                        message_id,
                    },
                    prompt: ollama::prompts::deep_search(query, results),
                });

                Result::<(), reqwest::Error>::Ok(())
            });
        }
    };

    Ok(())
}

fn find_url_in_reply(msg: &Message) -> Option<Url> {
    msg.reply_to_message()?
        .text()?
        .split_whitespace()
        .find_map(|word| Url::parse(word).ok())
}
