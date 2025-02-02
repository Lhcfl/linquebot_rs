//! todo command  
//! ```
//! /todo time thing
//! ```
//! Remind the user to do <thing> after <time> minutes. If the message is a reply, set the user as the repliee.

use log::{error, warn};
use msg_context::Context;
use std::time::Duration;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;

pub fn on_message(ctx: &mut Context, message: &Message) -> Consumption {
    let args = ctx.cmd?.content;
    let user = match message.reply_to_message() {
        Some(msg) => msg.from.as_ref(),
        None => message.from.as_ref(),
    }?
    .clone();
    let ctx = ctx.task();

    let (pre, Some(thing)) = split_n::<2>(args) else {
        return Consumption::StopWith(Box::pin(async move {
            if let Err(err) = ctx
                .reply("/todo 需要至少两个参数哦，第一个参数是分钟，第二个参数是琳酱要提醒干什么事")
                .send()
                .await
            {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }));
    };

    let [time] = pre[..] else {
        unreachable!(
            "split_n should return None for second result if pre have less than N elements"
        );
    };

    let Ok(time) = time.parse::<f64>() else {
        return Consumption::StopWith(Box::pin(async move {
            if let Err(err) = ctx.reply("没法解析出要几分钟后提醒呢").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }));
    };

    if time < 0.0 {
        return Consumption::StopWith(Box::pin(async move {
            if let Err(err) = ctx
                .reply("琳酱暂未研究出时间折跃技术，没法在过去提醒呢")
                .send()
                .await
            {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }));
    }

    if time > (365 * 24 * 60) as f64 {
        return Consumption::StopWith(Box::pin(async move {
            if let Err(err) = ctx.reply("太久远啦！").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }));
    }

    let thing = String::from(thing);

    Consumption::StopWith(Box::pin(async move {
        if let Err(err) = ctx
            .reply_html(format!(
                "设置成功！将在 {time} 分钟后提醒 {} {}",
                user.html_link(),
                escape_html(&thing)
            ))
            .send()
            .await
        {
            warn!("Failed to send reply: {}", err.to_string());
            return;
        }

        tokio::time::sleep(Duration::from_millis((time * 60.0 * 1000.0) as u64)).await;

        for retries in 0..3 {
            if let Err(err) = ctx
                .reply_html(format!(
                    "{} 该{}啦！",
                    &user.mention().as_ref().unwrap_or(&user.first_name),
                    escape_html(&thing)
                ))
                .send()
                .await
            {
                warn!(
                    "(retry {} times) Failed to send reply: {}",
                    retries,
                    err.to_string()
                );
            } else {
                return;
            }
        }

        error!("too many request errors, stop.")
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "todo",
        description: "定时提醒",
        description_detailed: Some(concat!(
            "必须有两个参数。使用 <code>/todo n [事情]</code> 在 n 分钟后提醒你做 [事情]。\n",
            "n 必须是数字。如果 n 太大的话，琳酱会拒绝提醒的"
        )),
    }),
    task: on_message,
};
