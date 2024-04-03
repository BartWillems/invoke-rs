use teloxide::{prelude::*, utils::command::BotCommands};

use crate::handler::local::{Notifier, Update};
use crate::handler::Store;
use crate::local_ai::Model;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    Hey(String),
    Oi(String),
    Tldr,
}

impl Command {
    fn override_prompt(&mut self, prompt: impl ToString) {
        match self {
            Command::Hey(_) => *self = Command::Hey(prompt.to_string()),
            Command::Oi(_) => *self = Command::Oi(prompt.to_string()),
            Command::Tldr => (),
        }
    }
}

pub async fn handler(
    bot: Bot,
    notifier: Notifier,
    msg: Message,
    mut command: Command,
    overrides: super::admin::Overrides,
    store: Store,
) -> Result<(), teloxide::RequestError> {
    log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);

    let user = match msg.from() {
        Some(user) => user,
        None => {
            log::warn!("Received a command without a user");
            return Ok(());
        }
    };

    if let Some(prompt) = overrides.get_override(user.id).await {
        command.override_prompt(prompt);
    }

    if user.id == UserId(172179034) {
        command.override_prompt("What are the hazards of driving on a flat tire?");
    }

    match command {
        Command::Hey(prompt) | Command::Oi(prompt) => notifier.notify(Update::Requested {
            chat_id: msg.chat.id,
            user_id: user.id,
            message_id: msg.id,
            prompt,
            model: Model::Llama,
        }),
        Command::Tldr => {
            let chat_history = match store.chat_history(&bot, msg.chat.id).await {
                Ok(Some(history)) => history,
                Ok(None) => {
                    bot.send_message(msg.chat.id, "No chat content found. Please let me learn longer or adjust my permissions.")
                        .await?;
                    return Ok(());
                }
                Err(err) => {
                    log::error!("failed to fetch chat history: {err}");
                    bot.send_message(msg.chat.id, "failure").await?;
                    return Ok(());
                }
            };

            // //             // TODO: actually figure out prompt
            // let prompt = "bart: ben naar de zoo gegaan
            // dieter: howla was het leuk
            // bart: ja heb apen gezien
            // gerben: wow daar heb ik bang van
            // hannes: kijk mijn nieuwe computer, ik ga zien of ik daar Hackintosh op kan installeren
            // hannes: also emilie heeft weer den auto kapot gereden, precies kenny
            // bart: hahahahha
            // Lisa: Hallo iedereen! Hoe gaat het vandaag?
            // Erik: Goedemorgen Lisa! Met mij gaat het prima, bedankt. Hoe is het met jou?
            // Lisa: Ook goed, dank je. Heeft iemand plannen voor het weekend?
            // Marieke: Ik ga naar het strand met wat vrienden. Jullie?
            // Erik: Ik denk dat ik een paar klusjes in huis moet doen, maar misschien kunnen we zondag samen ergens lunchen?
            // Lisa: Dat klinkt gezellig! Ik ben erbij.
            // Marieke: Leuk idee! Waar zullen we afspreken?
            // Erik: Wat dachten jullie van dat nieuwe café in de stad?
            // Lisa: Prima voor mij!
            // Marieke: Ja, dat klinkt goed. Hoe laat spreken we af?
            // Erik: Laten we zeggen rond 12 uur, is dat oké?
            // Lisa: Perfect, ik kijk er al naar uit!
            // Marieke: Ik ook! Het wordt vast een leuke dag.
            // Erik: Zeker weten! Hebben jullie nog iets interessants meegemaakt deze week?
            // Lisa: Ik heb een interessant boek gelezen over mindfulness. Heel inspirerend!
            // Marieke: Oh, dat klinkt interessant. Kun je me de titel doorgeven? Misschien wil ik het ook lezen.
            // Lisa: Natuurlijk! Het heet \"De Kracht van het Nu\" van Eckhart Tolle.
            // Erik: Klinkt goed, ik zal het ook op mijn leeslijst zetten.
            // Marieke: Bedankt voor de tip, Lisa!
            // Lisa: Graag gedaan. Hebben jullie nog plannen voor vanavond?
            // Erik: Ik denk dat ik gewoon thuis blijf en een film kijk. Jij, Marieke?
            // Marieke: Ik ga uit eten met mijn familie ter ere van mijn moeders verjaardag.
            // Lisa: Dat klinkt gezellig! Gefeliciteerd met je moeder.
            // Marieke: Dank je wel, Lisa!
            // Erik: Veel plezier vanavond, Marieke.
            // Marieke: Dank je, ik zal zeker genieten. En jullie ook veel plezier, Lisa en Erik, met jullie avond!
            // Lisa: Dank je wel, Marieke! Geniet van het etentje.
            // Erik: Tot volgende week dan, en fijn weekend allemaal!
            // Marieke: Tot volgende week! Fijn weekend!";

            notifier.notify(Update::Requested {
                chat_id: msg.chat.id,
                user_id: user.id,
                message_id: msg.id,
                prompt: chat_history,
                model: Model::Tldr,
            })
        }
    };

    Ok(())
}
