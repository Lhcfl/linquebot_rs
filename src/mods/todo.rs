//! todo command  
//! ```
//! /todo time thing
//! ```
//! Remind the user to do <thing> after <time> minutes. If the message is a reply, set the user as the repliee.

use log::warn;
use std::time::Duration;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;

async fn send_reply(app: &App, message: &Message, text: &str) {
    let res = app
        .bot
        .send_message(message.chat.id, text)
        .reply_parameters(ReplyParameters::new(message.id))
        .parse_mode(ParseMode::Html)
        .send()
        .await;
    if let Err(err) = res {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

pub fn on_message(app: &'static App, message: &Message) -> Consumption {
    let args = app.parse_command(message.text()?, "todo")?;
    let user = match message.reply_to_message() {
        Some(msg) => msg.from.as_ref(),
        None => message.from.as_ref(),
    }?
    .clone();

    let message = message.clone();

    let (pre, Some(thing)) = split_n::<2>(args) else {
        return Consumption::StopWith(Box::pin(async move {
            send_reply(
                app,
                &message,
                "/todo 需要至少两个参数哦，第一个参数是分钟，第二个参数是琳酱要提醒干什么事",
            )
            .await
        }));
    };

    let [time] = pre[..] else {
        unreachable!(
            "split_n should return None for second result if pre have less than N elements"
        );
    };

    let Ok(time) = time.parse::<f64>() else {
        return Consumption::StopWith(Box::pin(async move {
            send_reply(app, &message, "没法解析出要几分钟后提醒呢").await;
        }));
    };

    if time < 0.0 {
        return Consumption::StopWith(Box::pin(async move {
            send_reply(
                app,
                &message,
                "琳酱暂未研究出时间折跃技术，没法在过去提醒呢",
            )
            .await;
        }));
    }

    if time > (365 * 24 * 60) as f64 {
        return Consumption::StopWith(Box::pin(async move {
            send_reply(app, &message, "太久远啦！").await;
        }));
    }

    let thing = String::from(thing);

    return Consumption::StopWith(Box::pin(async move {
        send_reply(
            app,
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
            app,
            &message,
            &format!(
                "{} 该{}啦！",
                user.mention().unwrap_or(user.first_name),
                escape_html(&thing)
            ),
        )
        .await;
    }));
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "todo",
        description: "使用 `/todo [n] [事情]` 在 n 分钟后提醒你做事",
        description_detailed: None,
    }),
    task: on_message,
};
