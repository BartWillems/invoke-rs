use anyhow::bail;
use image::{DynamicImage, RgbaImage};
use rand::prelude::*;
use rand::seq::SliceRandom;
use tokio::fs;
use tokio::fs::read_dir;

use std::{io::Cursor, path::Path, sync::Arc};

use super::Context;
use teloxide::{prelude::*, types::InputFile, utils::command::BotCommands};

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    FactCHeck,
}

pub async fn handler(
    ctx: Context,
    msg: Message,
    command: Command,
) -> Result<(), teloxide::RequestError> {
    log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);
    match command {
        Command::FactCHeck => {
            let Some(original_message) = msg.reply_to_message() else {
                ctx.quick_reply(
                    &msg,
                    "What do you want me to fact check with 100% accuracy? (you have to reply to the message you want fact checked)",
                )
                .await;
                return Ok(());
            };

            let outcome = if original_message.date.timestamp() % 2 == 0 {
                FactCheck::Correct
            } else {
                FactCheck::FakeNews
            };

            let bytes = match ctx
                .fact_check_engine
                .get_random_fact_check_outcome(outcome)
                .await
            {
                Ok(bytes) => bytes,
                Err(err) => {
                    ctx.quick_reply(&msg, format!("something is broken: {err}"))
                        .await;
                    return Ok(());
                }
            };

            ctx.bot
                .send_photo(msg.chat.id, InputFile::memory(bytes))
                .reply_to_message_id(original_message.id)
                .send()
                .await?;
        }
    };

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum FactCheck {
    Correct,
    FakeNews,
}

struct Overlays {
    correct: Vec<u8>,
    fake_news: Vec<u8>,
}

impl Overlays {
    async fn from_path(path: &str) -> Result<Self, anyhow::Error> {
        let correct = fs::read(format!("{path}/overlays/correct.png")).await?;
        let fake_news = fs::read(format!("{path}/overlays/fake-news.png")).await?;

        Ok(Self { correct, fake_news })
    }

    fn get_overlay(&self, outcome: FactCheck) -> &[u8] {
        match outcome {
            FactCheck::Correct => &self.correct,
            FactCheck::FakeNews => &self.fake_news,
        }
    }
}

#[derive(Clone)]
pub struct Engine {
    path: Arc<String>,
    overlays: Arc<Overlays>,
}

impl Engine {
    const FAKE_NEWS_PATH: &'static str = "fake-news";
    const CORRECT_PATH: &'static str = "correct";

    pub async fn new(path: String) -> Result<Self, anyhow::Error> {
        let overlays = Overlays::from_path(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            overlays: Arc::new(overlays),
        })
    }

    pub async fn get_random_fact_check_outcome(
        &self,
        outcome: FactCheck,
    ) -> anyhow::Result<Vec<u8>> {
        let path = match outcome {
            FactCheck::Correct => format!("{}/{}", self.path, Self::CORRECT_PATH),
            FactCheck::FakeNews => format!("{}/{}", self.path, Self::FAKE_NEWS_PATH),
        };

        let dir = Path::new(&path);

        let mut entries = read_dir(dir).await?;
        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "png" || extension == "webp" {
                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                    }
                }
            }
        }

        let Some(random_picture_path) = files.choose(&mut thread_rng()) else {
            bail!("no pictures found");
        };

        let bytes = fs::read(random_picture_path).await?;

        let base = image::load_from_memory(&bytes)?.to_rgba8();

        let overlay = image::load_from_memory(self.overlays.get_overlay(outcome))?.to_rgba8();

        let result = Self::overlay_images(base, &overlay);

        let mut png_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut png_bytes);
        DynamicImage::ImageRgba8(result).write_to(&mut cursor, image::ImageFormat::Png)?;

        Ok(png_bytes)
    }

    fn overlay_images(mut base: RgbaImage, overlay: &RgbaImage) -> RgbaImage {
        for (x, y, overlay_px) in overlay.enumerate_pixels() {
            let base_px = base.get_pixel_mut(x, y);
            let alpha = overlay_px.0[3] as f32 / 255.0; // Normalize alpha (0-1)

            if alpha > 0.0 {
                // Skip fully transparent pixels
                for c in 0..3 {
                    // Blend RGB channels
                    base_px.0[c] = ((overlay_px.0[c] as f32 * alpha)
                        + (base_px.0[c] as f32 * (1.0 - alpha)))
                        as u8;
                }
                base_px.0[3] = 255; // Keep final image fully opaque
            }
        }

        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image_overlay() {
        let engine = Engine::new("patrioten".to_string()).await.unwrap();

        let res = engine
            .get_random_fact_check_outcome(FactCheck::Correct)
            .await
            .unwrap();

        tokio::fs::write("patrioten/test_output.png", res)
            .await
            .unwrap();
    }
}
