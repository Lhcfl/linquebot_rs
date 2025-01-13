use colored::Colorize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::ComsumedType;

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
    match result {
        Ok(_) => {}
        Err(err) => {
            println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
        }
    }
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let text = message.text()?;

    // let message = message;
    let chat_id = message.chat.id;
    let mut actee = message.reply_to_message().as_ref()?.from.clone()?;
    let mut actor = message.from.as_ref()?.clone();

    if text.starts_with('\\') {
        (actee, actor) = (actor, actee);
    } else if !text.starts_with('/') {
        return None;
    }

    let text = String::from(text);
    let mut iter = text[1..].split_whitespace();
    let action = iter.next()?.to_string();
    let addition = iter.next().and_then(|str| Some(str.to_string()));
    let bot = bot.clone();

    tokio::spawn(async move { send_rong(bot, chat_id, actor, actee, action, addition).await });

    Some(ComsumedType::Next)
}
