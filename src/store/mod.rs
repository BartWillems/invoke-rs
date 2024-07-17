use std::fmt::Write;
use std::time::Duration;
use std::writeln;

use async_trait::async_trait;
use moka::future::Cache;
use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::{Sqlite, SqlitePool};
use teloxide::prelude::*;
use teloxide::{types::ChatId, Bot};

use crate::ollama;

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone)]
pub struct Store<U: UsernameProvider = Bot> {
    sqlite: SqlitePool,
    cache: Cache<UserId, String>,
    usernames: U,
    model: ollama::Model,
}

#[async_trait]
pub trait UsernameProvider {
    async fn get_username(
        &self,
        chat_id: ChatId,
        user_id: UserId,
        cache: &Cache<UserId, String>,
    ) -> Result<String, anyhow::Error>;
}

#[async_trait]
impl UsernameProvider for Bot {
    async fn get_username(
        &self,
        chat_id: ChatId,
        user_id: UserId,
        cache: &Cache<UserId, String>,
    ) -> Result<String, anyhow::Error> {
        if let Some(username) = cache.get(&user_id).await {
            return Ok(username);
        };

        let member = self.get_chat_member(chat_id, user_id).await?;

        let username = member
            .user
            .username
            .as_ref()
            .map(|username| format!("@{username}"))
            .unwrap_or(member.user.full_name());

        cache.insert(user_id, username.clone()).await;

        Ok(username)
    }
}

impl<U: UsernameProvider> Store<U> {
    const TLDR_PROMPT: &'static str = "The text below is a chat conversation. Each message is in the format of \"sender-name: message-content\". Respond only with a recap of what each person has said/done in the conversation. Always tag the users' usernames by prefixing an '@' before their name in your recap.";

    pub async fn new(
        url: &str,
        usernames: U,
        model: ollama::Model,
    ) -> Result<Store<U>, anyhow::Error> {
        if !Sqlite::database_exists(url).await.unwrap_or(false) {
            Sqlite::create_database(url).await?;
        }

        let sqlite = SqlitePool::connect(url).await?;

        MIGRATOR.run(&sqlite).await?;

        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(60 * 15))
            .build();

        Ok(Self {
            sqlite,
            cache,
            usernames,
            model,
        })
    }

    #[cfg(test)]
    pub async fn new_in_memory(usernames: U) -> Result<Store<U>, anyhow::Error> {
        let sqlite = SqlitePool::connect("sqlite::memory:").await?;

        MIGRATOR.run(&sqlite).await?;

        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(60 * 15))
            .build();

        Ok(Self {
            sqlite,
            cache,
            usernames,
            model: ollama::Model::default(),
        })
    }

    pub async fn chat_history(&self, chat_id: ChatId) -> Result<Option<String>, anyhow::Error> {
        let messages: Vec<(i64, String)> = sqlx::query_as(
            r#"
            SELECT user_id, message
            FROM chat_messages
            WHERE chat_id = $1
              AND created_at > datetime('now', '-8 hours')
              AND message IS NOT NULL
            ORDER BY created_at DESC"#,
        )
        .bind(chat_id.0)
        .fetch_all(&self.sqlite)
        .await?;

        if messages.is_empty() {
            return Ok(None);
        }

        let max_context_size = self.model.context_length();
        let mut context_size = Self::TLDR_PROMPT.len();
        let mut buffer = Vec::with_capacity(messages.len());

        for (user_id, message) in messages
            .into_iter()
            .map(|(user_id, message)| (UserId(user_id as u64), message))
        {
            let username = self
                .usernames
                .get_username(chat_id, user_id, &self.cache)
                .await?;

            let line = format!("{username}: {message}");

            if context_size + line.len() > max_context_size {
                log::info!("max context size reached");
                break;
            }

            context_size += line.len();
            buffer.push(line);
        }

        let mut chat_content = String::with_capacity(context_size);

        writeln!(chat_content, "{}", Self::TLDR_PROMPT)?;

        for line in buffer.into_iter().rev() {
            writeln!(chat_content, "{line}")?;
        }

        Ok(Some(chat_content))
    }

    pub async fn store_message(&self, msg: Message) -> Result<(), anyhow::Error> {
        let user = msg
            .from()
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;

        let message_text = match msg.text() {
            Some("") => {
                log::debug!("skipping empty message");
                return Ok(());
            }
            Some(msg) => msg,
            None => {
                log::debug!("skipping non-text message");
                return Ok(());
            }
        };

        sqlx::query(
            r#"
        INSERT INTO chat_messages
        (chat_id, user_id, message_id, message)
        VALUES ($1, $2, $3, $4)
        "#,
        )
        .bind(msg.chat.id.0)
        .bind(user.id.0 as i64)
        .bind(msg.id.0)
        .bind(message_text)
        .execute(&self.sqlite)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::atomic::AtomicI32};

    use chrono::Utc;
    use teloxide::types::{
        Chat, ChatPrivate, MediaKind, MediaText, MessageCommon, MessageId, MessageKind, User,
    };

    use super::*;

    static ID_COUNTER: AtomicI32 = AtomicI32::new(1);

    struct UsernameStore {
        store: HashMap<UserId, String>,
    }

    impl UsernameStore {
        pub fn new(content: impl Into<HashMap<UserId, String>>) -> Self {
            Self {
                store: content.into(),
            }
        }
    }

    #[async_trait]
    impl UsernameProvider for UsernameStore {
        async fn get_username(
            &self,
            _: ChatId,
            user_id: UserId,
            _: &Cache<UserId, String>,
        ) -> Result<String, anyhow::Error> {
            Ok(self
                .store
                .get(&user_id)
                .cloned()
                .unwrap_or(String::from("default")))
        }
    }

    #[tokio::test]
    async fn test_foo() {
        let store = Store::new_in_memory(UsernameStore::new([
            (UserId(1), String::from("@don_johnson")),
            (UserId(15), String::from("@bovine_von_johnson")),
        ]))
        .await
        .unwrap();

        let message = generate_message(ChatId(1), UserId(1), "first message");
        store.store_message(message).await.unwrap();

        tokio::time::sleep(Duration::from_millis(2000)).await;

        let message = generate_message(ChatId(1), UserId(15), "second message");
        store.store_message(message).await.unwrap();

        let chat_history = store.chat_history(ChatId(1)).await.unwrap();

        assert_eq!(
            chat_history,
            Some(String::from(
                "@don_johnson: first message\n@bovine_von_johnson: second message\n"
            ))
        );
    }

    fn generate_message(chat_id: ChatId, user_id: UserId, content: impl Into<String>) -> Message {
        Message {
            id: MessageId(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Release)),
            thread_id: None,
            date: Utc::now(),
            chat: Chat {
                id: chat_id,
                kind: teloxide::types::ChatKind::Private(ChatPrivate {
                    username: None,
                    first_name: None,
                    last_name: None,
                    emoji_status_custom_emoji_id: None,
                    bio: None,
                    has_private_forwards: None,
                    has_restricted_voice_and_video_messages: None,
                }),
                photo: None,
                pinned_message: None,
                message_auto_delete_time: None,
                has_hidden_members: false,
                has_aggressive_anti_spam_enabled: false,
            },
            via_bot: None,
            kind: MessageKind::Common(MessageCommon {
                from: Some(User {
                    id: user_id,
                    is_bot: false,
                    first_name: "Bon".into(),
                    last_name: None,
                    username: None,
                    language_code: None,
                    is_premium: false,
                    added_to_attachment_menu: false,
                }),
                sender_chat: None,
                author_signature: None,
                forward: None,
                reply_to_message: None,
                edit_date: None,
                media_kind: MediaKind::Text(MediaText {
                    text: content.into(),
                    entities: Vec::new(),
                }),
                reply_markup: None,
                is_topic_message: false,
                is_automatic_forward: false,
                has_protected_content: false,
            }),
        }
    }
}
