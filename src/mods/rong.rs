//! 实现 @rongslashbot 的功能

use std::collections::HashSet;
use std::sync::LazyLock;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::msg_context::Context;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;
use crate::Module;
use crate::ModuleDescription;
use crate::ModuleKind;

// 常见其它 bot 的命令名单，防止意外回复
static RONG_BLACKLIST: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "pin", "q", "dedede", "hammer", "start", "quit", "search", "close", "open", "join", "kill",
        "kick", "settings", "enable", "disable", "leave", "skip",
    ])
});

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

    let [action, addition] = split_args(&text[1..]);
    if action.is_empty() {
        return Consumption::Next;
    }
    if RONG_BLACKLIST.contains(&action) {
        return Consumption::Next;
    }

    let mut reply = format!(
        "{} {} {}",
        actor.html_link(),
        escape_html(&action),
        if actor.id == actee.id {
            format!("<a href=\"{}\">自己</a>", actee.preferably_tme_url())
        } else {
            actee.html_link()
        }
    );

    if !addition.is_empty() {
        reply.push(' ');
        reply.push_str(&escape_html(&addition));
    }

    reply.push('!');

    let ctx = ctx.task();

    ctx.reply_html(reply)
        .link_preview_options(LinkPreviewOptions {
            is_disabled: true,
            url: None,
            prefer_large_media: false,
            prefer_small_media: false,
            show_above_text: false,
        })
        .send()
        .warn_on_error("rong")
        .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::General(Some(ModuleDescription {
        name: "rong",
        description: "Rong一下人",
        description_detailed: Some(concat!(
            "对于没有在模块记录内的命令，如果你对某个人回复 <code>/动作 短语</code>，会回复“你 动作 某个人 短语！”"
        )),
    })),
    task: rong,
};
