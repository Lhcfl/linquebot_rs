use std::time::Duration;

use colored::Colorize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::ComsumedType;

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let text = message.text()?;
    if !has_command(text, "todo") {
        return None;
    }

    let bot = bot.clone();
    let user = message.clone().from?;
    let message = message.clone();

    let (pre, Some(thing)) = split_n::<3>(text) else {
        tokio::spawn(async move {
            let res = bot
                .send_message(
                    message.chat.id,
                    "/todo 需要至少两个参数哦，第一个参数是分钟，第二个参数是琳酱要提醒干什么事",
                )
                .reply_parameters(ReplyParameters::new(message.id))
                .send()
                .await;
            if let Err(err) = res {
                println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
            }
        });
        return Some(ComsumedType::Stop);
    };

    let [_, time] = pre[..] else {
        unreachable!(
            "split_n should return None for second result if pre have less than N elements"
        );
    };

    let Ok(time) = time.parse::<f64>() else {
        tokio::spawn(async move {
            let res = bot
                .send_message(message.chat.id, "没法解析出要几分钟后提醒呢")
                .reply_parameters(ReplyParameters::new(message.id))
                .send()
                .await;
            if let Err(err) = res {
                println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
            }
        });
        return Some(ComsumedType::Stop);
    };

    let thing = String::from(thing);

    tokio::spawn(async move {
        let res = bot
            .send_message(
                message.chat.id,
                format!(
                    "设置成功！将在 {time} 分钟后提醒 {} {}",
                    user.html_link(),
                    escape_html(&thing)
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(message.id))
            .send()
            .await;
        if let Err(err) = res {
            println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
        }

        tokio::time::sleep(Duration::from_millis((time * 60.0 * 1000.0) as u64)).await;

        let res = bot
            .send_message(
                message.chat.id,
                format!(
                    "{} 该{}啦！",
                    user.mention().unwrap_or(user.first_name),
                    escape_html(&thing)
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(message.id))
            .send()
            .await;
        if let Err(err) = res {
            println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
        }
    });

    Some(ComsumedType::Stop)
}
