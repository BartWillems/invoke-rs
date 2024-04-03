use std::collections::HashMap;
use std::fmt::Write;
use std::writeln;

use chrono::{Days, Utc};
use sqlx::migrate::MigrateDatabase;
use sqlx::{migrate, Sqlite, SqlitePool};
use teloxide::prelude::*;
use teloxide::{types::ChatId, Bot};

#[derive(Clone)]
pub struct Store {
    sqlite: SqlitePool,
}

impl Store {
    pub async fn new(url: &str) -> Result<Self, anyhow::Error> {
        let migrations = migrate!("./migrations");

        if !Sqlite::database_exists(url).await.unwrap_or(false) {
            Sqlite::create_database(url).await?;
        }

        let sqlite = SqlitePool::connect(url).await?;

        migrations.run(&sqlite).await?;

        Ok(Self { sqlite })
    }

    pub async fn chat_history(
        &self,
        bot: &Bot,
        chat_id: ChatId,
    ) -> Result<Option<String>, anyhow::Error> {
        // TODO: cache?
        let mut usernames: HashMap<UserId, String> = HashMap::new();

        let messages: Vec<(i64, String)> = sqlx::query_as(
            r#"
            SELECT user_id, message
            FROM chat_messages
            WHERE chat_id = $1
              AND created_at > $2
              AND message IS NOT NULL"#,
        )
        .bind(chat_id.0)
        .bind(Utc::now() - Days::new(1))
        .fetch_all(&self.sqlite)
        .await?;

        if messages.is_empty() {
            return Ok(None);
        }

        let mut chat_content = String::with_capacity(messages.len() * 20);

        for (user_id, message) in messages {
            let user_id = UserId(user_id as u64);

            if user_id == UserId(172179034) {
                continue;
            }

            let username = match usernames.get(&user_id) {
                Some(username) => username.clone(),
                None => {
                    let member = bot.get_chat_member(chat_id, user_id).await?;
                    let username = member
                        .user
                        .username
                        .as_ref()
                        .map(|username| format!("@{username}"))
                        .unwrap_or(member.user.full_name());

                    usernames.insert(user_id, username.clone());
                    username
                }
            };

            writeln!(chat_content, "{username}: {message}")?;
        }

        Ok(Some(chat_content))
    }

    pub async fn store_message(&self, msg: Message) -> Result<(), anyhow::Error> {
        let user = msg
            .from()
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;

        let message_text = match msg.text() {
            Some(msg) if msg.is_empty() => {
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
        .bind(msg.chat.id.0 as i64)
        .bind(user.id.0 as i64)
        .bind(msg.id.0)
        .bind(message_text)
        .execute(&self.sqlite)
        .await?;

        Ok(())
    }
}
