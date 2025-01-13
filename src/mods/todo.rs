use std::time::Duration;

use colored::Colorize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::ComsumedType;

async fn send_reply(bot: &Bot, message: &Message, text: &str) {
    let res = bot
        .send_message(message.chat.id, text)
        .reply_parameters(ReplyParameters::new(message.id))
        .parse_mode(ParseMode::Html)
        .send()
        .await;
    if let Err(err) = res {
        println!("{}: RequestError: {}", "warn".yellow(), err.to_string());
    }
}

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let text = message.text()?;
    if !has_command(text, "todo") {
        return None;
    }

    let bot = bot.clone();

    let user = match message.reply_to_message() {
        Some(msg) => msg.from.as_ref(),
        None => message.from.as_ref(),
    }?
    .clone();

    let message = message.clone();

    let (pre, Some(thing)) = split_n::<3>(text) else {
        tokio::spawn(async move {
            send_reply(
                &bot,
                &message,
                "/todo 需要至少两个参数哦，第一个参数是分钟，第二个参数是琳酱要提醒干什么事",
            )
            .await
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
            send_reply(&bot, &message, "没法解析出要几分钟后提醒呢").await;
        });
        return Some(ComsumedType::Stop);
    };

    if time < 0.0 {
        tokio::spawn(async move {
            send_reply(
                &bot,
                &message,
                "琳酱暂未研究出时间折跃技术，没法在过去提醒呢",
            )
            .await;
        });
        return Some(ComsumedType::Stop);
    }

    if time > (365 * 24 * 60) as f64 {
        tokio::spawn(async move {
            send_reply(&bot, &message, "太久远啦！").await;
        });
        return Some(ComsumedType::Stop);
    }

    let thing = String::from(thing);

    tokio::spawn(async move {
        send_reply(
            &bot,
            &message,
            &format!(
                "设置成功！将在 {time} 分钟后提醒 {} {}",
                user.html_link(),
                escape_html(&thing)
            ),
        )
        .await;

        tokio::time::sleep(Duration::from_millis((time * 60.0 * 1000.0) as u64)).await;

        send_reply(
            &bot,
            &message,
            &format!(
                "{} 该{}啦！",
                user.mention().unwrap_or(user.first_name),
                escape_html(&thing)
            ),
        )
        .await;
    });

    Some(ComsumedType::Stop)
}
