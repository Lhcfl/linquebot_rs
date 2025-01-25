use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::msg_context::Context;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;
use crate::Module;
use crate::ModuleDesctiption;
use crate::ModuleKind;

pub fn rong(ctx: &mut Context, message: &Message) -> Consumption {
    let text = message.text()?;
    if let Some(username) = ctx.cmd.and_then(|cmd| cmd.username) {
        if username != ctx.app.username {
            return Consumption::Next;
        }
    }

    let mut actee = message.reply_to_message().as_ref()?.from.clone()?;
    let mut actor = message.from.as_ref()?.clone();

    if text.starts_with('\\') {
        (actee, actor) = (actor, actee);
    } else if !text.starts_with('/') {
        return Consumption::Next;
    }

    let text = String::from(text);
    let mut iter = text[1..].split_whitespace();
    let action = iter.next()?.to_string();
    let addition = iter
        .remainder()
        .and_then(|str| Some(str.trim().to_string()));
    let ctx = ctx.task();

    Consumption::StopWith(Box::pin(async move {
        let mut text = format!(
            "{} {} {}",
            actor.html_link(),
            escape_html(&action),
            if actor.id == actee.id {
                format!("<a href=\"{}\">自己</a>", actee.preferably_tme_url())
            } else {
                actee.html_link()
            }
        );
        if let Some(addition) = addition {
            text.push(' ');
            text.push_str(&escape_html(&addition));
        }
        text.push('!');

        let result = ctx
            .reply_html(&text)
            .link_preview_options(LinkPreviewOptions {
                is_disabled: true,
                url: None,
                prefer_large_media: false,
                prefer_small_media: false,
                show_above_text: false,
            })
            .send()
            .await;
        if let Err(err) = result {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::General(Some(ModuleDesctiption {
        name: "rong",
        description: "Rong一下人",
        description_detailed: Some(concat!(
            "对于没有在模块记录内的命令，如果你对某个人回复 <code>/动作 短语</code>，会回复“你 动作 某个人 短语！”"
        )),
    })),
    task: rong,
};
