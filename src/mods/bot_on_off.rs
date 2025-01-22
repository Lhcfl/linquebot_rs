//! 这还是个雏形
use log::warn;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::Consumption;

use crate::utils::parse_command;

static BOT_ON: LazyLock<RwLock<HashMap<ChatId, bool>>> = LazyLock::new(Default::default);

fn on_bot_on_message(bot: &Bot, message: &Message) -> Consumption {
    let _ = parse_command(message.text()?, "bot_on")?;
    let chat_id = message.chat.id;
    BOT_ON.write().unwrap().insert(chat_id, true);

    let bot = bot.clone();
    tokio::spawn(async move {
        let res = bot.send_message(chat_id, "琳酱已开机").send().await;
        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    });

    Consumption::Stop
}
fn on_bot_off_message(bot: &Bot, message: &Message) -> Consumption {
    let _ = parse_command(message.text()?, "bot_off")?;
    let chat_id = message.chat.id;
    BOT_ON.write().unwrap().insert(chat_id, false);

    let bot = bot.clone();
    let chat_id = message.chat.id;
    tokio::spawn(async move {
        let res = bot.send_message(chat_id, "琳酱已关机").send().await;
        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    });

    Consumption::Stop
}

// pub fn on_message(bot: &Bot, message: &Message) -> Consumption {
//     if BOT_ON
//         .read()
//         .unwrap()
//         .get(&message.chat.id)
//         .cloned()
//         .unwrap_or(true)
//     {
//         on_bot_off_message(bot, message).or(on_bot_on_message(bot, message))
//     } else {
//         let _ = on_bot_off_message(bot, message);
//         let _ = on_bot_on_message(bot, message);
//         Consumption::Stop
//     }
// }
