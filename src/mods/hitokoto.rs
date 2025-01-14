//! hitokoto  
//! Send a hitokoto  
//! Usage:
//! ```
//! /hitokoto
//! ```

use colored::Colorize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::*;
use crate::ComsumedType;

const API_ADDRESS: &str = "https://v1.hitokoto.cn/?c=";

struct Hitokoto {
    hitokoto: String,
    from: String,
}

async fn get_hitokoto(args: &str) -> Hitokoto {
    // todo
    Hitokoto {
        hitokoto: "网络错误".to_string(),
        from: "琳酱".to_string(),
    }
}

async fn hitokoto(bot: &Bot, chat_id: ChatId, message_id: MessageId, args: String) {
    let res = get_hitokoto(&args).await;

    let res = bot
        .send_message(chat_id, format!("{} ——{}", res.hitokoto, res.from))
        .reply_parameters(ReplyParameters::new(message_id))
        .send()
        .await;

    if let Err(err) = res {
        println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
    }
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let args = parse_command(message.text()?, "hitokoto")?;
    let args = args.split_whitespace().collect::<Vec<_>>().join("&c=");
    let bot = bot.clone();
    let chat_id = message.chat.id;
    let message_id = message.id;

    tokio::spawn(async move {
        hitokoto(&bot, chat_id, message_id, args).await;
    });

    Some(ComsumedType::Stop)
}
