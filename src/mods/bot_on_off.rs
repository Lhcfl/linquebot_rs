//! 这还是个雏形

use colored::Colorize;
use std::sync::atomic::AtomicBool;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::ComsumedType;

use crate::utils::parse_command;

static BOT_ON: AtomicBool = AtomicBool::new(true);

pub fn on_bot_on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let _ = parse_command(message.text()?, "bot_on")?;
    BOT_ON.store(true, std::sync::atomic::Ordering::Relaxed);

    let bot = bot.clone();
    let chat_id = message.chat.id;
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
    BOT_ON.store(false, std::sync::atomic::Ordering::Relaxed);

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
    if BOT_ON.load(std::sync::atomic::Ordering::Relaxed) {
        on_bot_off_message(bot, message).or(on_bot_on_message(bot, message))
    } else {
        let _ = on_bot_off_message(bot, message);
        let _ = on_bot_on_message(bot, message);
        Some(ComsumedType::Stop)
    }
}
