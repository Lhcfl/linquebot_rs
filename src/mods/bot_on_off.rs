//! 这还是个雏形

use colored::Colorize;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::ComsumedType;

use crate::utils::parse_command;

static BOT_ON: LazyLock<RwLock<HashMap<ChatId, bool>>> = LazyLock::new(Default::default);

pub fn on_bot_on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let _ = parse_command(message.text()?, "bot_on")?;
    let chat_id = message.chat.id;
    BOT_ON.write().unwrap().insert(chat_id, true);

    let bot = bot.clone();
    tokio::spawn(async move {
        let res = bot.send_message(chat_id, "琳酱已开机").send().await;
        if let Err(err) = res {
            println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
        }
    });

    Some(ComsumedType::Stop)
}
pub fn on_bot_off_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let _ = parse_command(message.text()?, "bot_off")?;
    let chat_id = message.chat.id;
    BOT_ON.write().unwrap().insert(chat_id, false);

    let bot = bot.clone();
    let chat_id = message.chat.id;
    tokio::spawn(async move {
        let res = bot.send_message(chat_id, "琳酱已关机").send().await;
        if let Err(err) = res {
            println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
        }
    });

    Some(ComsumedType::Stop)
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    if BOT_ON
        .read()
        .unwrap()
        .get(&message.chat.id)
        .cloned()
        .unwrap_or(true)
    {
        on_bot_off_message(bot, message).or(on_bot_on_message(bot, message))
    } else {
        let _ = on_bot_off_message(bot, message);
        let _ = on_bot_on_message(bot, message);
        Some(ComsumedType::Stop)
    }
}
