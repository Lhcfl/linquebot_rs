use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::App;
use crate::Consumption;
use crate::Module;
use crate::ModuleDesctiption;
use crate::ModuleKind;

async fn send_rong(
    bot: Bot,
    chat_id: ChatId,
    actor_user: User,
    actee_user: User,
    action: String,
    addition: Option<String>,
) {
    let mut text = format!(
        "{} {} {}",
        actor_user.html_link(),
        escape_html(&action),
        actee_user.html_link(),
    );
    if let Some(addition) = addition {
        text.push(' ');
        text.push_str(&escape_html(&addition));
    }
    text.push('!');

    let result = bot
        .parse_mode(ParseMode::Html)
        .send_message(chat_id, text)
        .send()
        .await;

    if let Err(err) = result {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

pub fn on_message(app: &App, message: &Message) -> Consumption {
    let text = message.text()?;

    // let message = message;
    let chat_id = message.chat.id;
    let mut actee = message.reply_to_message().as_ref()?.from.clone()?;
    let mut actor = message.from.as_ref()?.clone();

    if text.starts_with('\\') {
        (actee, actor) = (actor, actee);
    } else if !text.starts_with('/') {
        return Consumption::Next;
    }

    let text = String::from(text);
    let mut iter = text[1..].split_whitespace();
    let action = iter.next()?.to_string();
    let addition = iter.next().and_then(|str| Some(str.to_string()));
    let bot = app.bot.clone();

    tokio::spawn(async move { send_rong(bot, chat_id, actor, actee, action, addition).await });

    Consumption::Stop
}

pub static MODULE: Module = Module {
    kind: ModuleKind::General(Some(ModuleDesctiption {
        name: "rong",
        description: "Rong一下人",
        description_detailed: None,
    })),
    task: on_message,
};
