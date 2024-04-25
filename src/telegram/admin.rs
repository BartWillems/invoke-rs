use std::{collections::HashMap, sync::Arc};

use teloxide::{
    macros::BotCommands,
    requests::Requester,
    types::{Message, User, UserId},
};
use tokio::sync::RwLock;

use super::Context;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Admin only commands")]
pub enum AdminCommands {
    #[command(description = "Give someone the clowns")]
    Clown,
    #[command(description = "Remove the clowns from someone")]
    UnClown,
    #[command(description = "set the LLM to drunk mode")]
    DrunkLlm,
    #[command(description = "revert the LLM prompt to default")]
    DefaultLlm,
    #[command(description = "Use a custom LLM system prompt")]
    CustomLlm(String),
}

#[derive(Clone, Default)]
pub struct Overrides {
    overrides: Arc<RwLock<HashMap<UserId, &'static str>>>,
}

impl Overrides {
    /// Get the override for a user if there exists one
    pub(crate) async fn get_override(&self, user_id: UserId) -> Option<&'static str> {
        self.overrides.read().await.get(&user_id).copied()
    }

    pub(crate) async fn set_override(&self, user_id: UserId, prompt: &'static str) {
        self.overrides.write().await.insert(user_id, prompt);
    }

    pub(crate) async fn remove_override(&self, user_id: UserId) {
        self.overrides.write().await.remove(&user_id);
    }
}

pub async fn handler(
    ctx: Context,
    msg: Message,
    cmd: AdminCommands,
    overrides: Overrides,
) -> Result<(), teloxide::RequestError> {
    match cmd {
        AdminCommands::Clown => {
            let target_user = match target_user(&msg) {
                Some(id) => id,
                None => {
                    log::warn!("unable to retriever replied user");
                    return Ok(());
                }
            };

            let username = target_user
                .mention()
                .unwrap_or_else(|| target_user.full_name());

            log::info!(
                "Added clownmode for '{username}', UserId({})",
                target_user.id
            );
            overrides
                .set_override(target_user.id, "a silly homeless drunk clown")
                .await;

            ctx.bot
                .send_message(msg.chat.id, format!("{username} has been clowned"))
                .await?;
        }
        AdminCommands::UnClown => {
            let target_user = match target_user(&msg) {
                Some(id) => id,
                None => {
                    log::warn!("unable to retriever replied user id");
                    return Ok(());
                }
            };

            let username = target_user
                .mention()
                .unwrap_or_else(|| target_user.full_name());
            log::info!(
                "Removed clownmode from '{username}', UserId({})",
                target_user.id
            );
            overrides.remove_override(target_user.id).await;

            ctx.bot
                .send_message(msg.chat.id, format!("{username} has been unclowned"))
                .await?;
        }
        AdminCommands::DefaultLlm => {
            ctx.prompts.reset().await;
        }
        AdminCommands::CustomLlm(prompt) => {
            ctx.prompts.overwrite_prompt(prompt).await;
        }
        AdminCommands::DrunkLlm => {
            ctx.prompts.overwrite_to_drunk().await;
        }
    };

    Ok(())
}

fn target_user(msg: &Message) -> Option<&User> {
    msg.reply_to_message()?.from()
}
