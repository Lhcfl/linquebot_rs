use colored::Colorize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;
use teloxide_core::ApiError;
use teloxide_core::RequestError;

use crate::utils::*;
use crate::ComsumedType;

async fn reply_info(bot: Bot, message: Message, info: &str) {
    if let Err(err) = bot
        .send_message(message.chat.id, info)
        .reply_parameters(ReplyParameters::new(message.id))
        .send()
        .await
    {
        println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
    }
}

async fn handle_err(err: RequestError, bot: Bot, message: Message) {
    println!("Err: {:?}", err);
    match err {
        RequestError::Api(ApiError::CantDemoteChatCreator) => {
            reply_info(bot, message, "不能给群主设置头衔哦").await
        }
        RequestError::Api(ApiError::Unknown(str)) => {
            if str.contains("can't remove chat owner") {
                reply_info(bot, message, "不能给群主设置头衔哦").await
            } else if str.contains("not enough rights") {
                reply_info(bot, message, "琳酱还没有这个权限哦").await
            } else {
                reply_info(bot, message, "因为未知错误而失败……").await
            }
        }
        _ => reply_info(bot, message, "因为未知错误而失败……").await,
    }
}
async fn clear_title(bot: Bot, message: Message, user: User) {
    if let Err(err) = bot
        .promote_chat_member(message.chat.id, user.id)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
    } else {
        reply_info(bot, message, "清除头衔成功！").await;
    }
}

async fn set_title(
    bot: Bot,
    message: Message,
    user: User,
    title: String,
) -> Result<(), &'static str> {
    if title.chars().count() >= 16 {
        reply_info(bot, message, "你想要的头衔太长了哦").await;
        return Err("TITLE_TOO_LONG");
    }

    if let Err(err) = bot
        .promote_chat_member(message.chat.id, user.id)
        .can_pin_messages(true)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
        return Err("REQUEST_ERR");
    }

    if let Err(err) = bot
        .set_chat_administrator_custom_title(message.chat.id, user.id, &title)
        .send()
        .await
    {
        handle_err(err, bot, message).await;
        return Err("REQUEST_ERR");
    }

    reply_info(bot, message, &format!("设置成功，现在你是 {title} 了")).await;
    Ok(())
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let text = message.text()?;
    if !has_command(text, "t") {
        return None;
    }

    let bot = bot.clone();
    let message = message.clone();
    let user = message.clone().from?;
    let text = String::from(&text[2..]);

    if !message.chat.is_group() && !message.chat.is_supergroup() {
        tokio::spawn(async move {
            reply_info(bot, message, "需要在群里才能设置头衔哦").await;
        });
    } else if text.trim().is_empty() {
        tokio::spawn(async move { clear_title(bot, message, user).await });
    } else {
        tokio::spawn(async move { set_title(bot, message, user, text).await });
    }

    Some(ComsumedType::Stop)
}
