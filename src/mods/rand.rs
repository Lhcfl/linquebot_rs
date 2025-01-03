use colored::Colorize;
use rand::seq::IteratorRandom;
use rand::Rng;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::ComsumedType;

async fn send_raw_rand(bot: Bot, message: Message, text_body: String) {
    let result = rand::thread_rng().gen_range(0..=100);

    let Some(from) = message.from.clone() else {
        println!("{}: Rand: No reply target", "warn".yellow());
        return;
    };

    let msg = format!(
        "{} {}",
        from.html_link(),
        if text_body.trim().is_empty() {
            format!("掷出了: {result}")
        } else {
            format!("{} 的概率是: {result}%", escape_html(text_body.trim()))
        }
    );

    if let Err(err) = bot
        .send_message(message.chat.id, msg)
        .parse_mode(ParseMode::Html)
        .send()
        .await
    {
        println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
    }
}

async fn send_selective_rand(
    bot: Bot,
    message: Message,
    text_body: String,
    spliter: &str,
) -> Option<()> {
    let result = text_body.split(&spliter).choose(&mut rand::thread_rng())?;

    if let Err(err) = bot
        .send_message(message.chat.id, format!("{}!", escape_html(result)))
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(message.id))
        .send()
        .await
    {
        println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
    }

    Some(())
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let text = message.text()?;
    if !text.starts_with("/rand") {
        return None;
    }

    let bot = bot.clone();
    let message = message.clone();
    let text = String::from(&text[5..]);

    if text.contains("还是") {
        tokio::spawn(async move { send_selective_rand(bot, message, text, "还是").await });
    } else {
        tokio::spawn(async move { send_raw_rand(bot, message, text).await });
    }

    Some(ComsumedType::Stop)
}
