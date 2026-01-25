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

#[derive(Debug)]
struct RongUser {
    turl: Option<String>,
    name: String,
}

impl RongUser {
    fn from_user(user: &User) -> Self {
        Self {
            name: user.full_name(),
            turl: Some(user.preferably_tme_url().to_string()),
        }
    }

    fn from_chat(chat: &Chat) -> Option<Self> {
        Some(Self {
            name: chat.title()?.to_string(),
            turl: chat.username().map(|name| format!("t.me/{name}")),
        })
    }

    fn html_link(&self) -> String {
        match &self.turl {
            Some(url) => format!("<a href=\"{url}\">{}</a>", escape_html(&self.name)),
            None => format!("<b>{}</b>", self.name),
        }
    }
}

pub fn rong(ctx: &mut Context, message: &Message) -> Consumption {
    let text = message.text()?;
    if let Some(username) = ctx.cmd.and_then(|cmd| cmd.username)
        && username != ctx.app.username {
            return Consumption::just_next();
        }

    let mut actee = message.reply_to_message().and_then(|msg| match &msg.from {
        Some(user) if !user.is_telegram() => Some(RongUser::from_user(user)),
        _ => RongUser::from_chat(msg.sender_chat.as_ref()?),
    })?;
    let mut actor = RongUser::from_user(message.from.as_ref()?);

    if text.starts_with('\\') {
        (actee, actor) = (actor, actee);
    } else if !text.starts_with('/') {
        return Consumption::just_next();
    }

    if actor.turl == actee.turl {
        actee.name = "自己".to_string()
    }

    let [action, addition] = split_args(&text[1..]);
    let action = action.trim_end_matches(&ctx.app.username);
    if action.is_empty() {
        return Consumption::just_next();
    }
    if RONG_BLACKLIST.contains(&action) {
        return Consumption::just_next();
    }

    let mut reply = format!(
        "{} {} {}",
        actor.html_link(),
        escape_html(action),
        actee.html_link(),
    );

    if !addition.is_empty() {
        reply.push(' ');
        reply.push_str(&escape_html(addition));
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
        description_detailed: Some("对于没有在模块记录内的命令，如果你对某个人回复 <code>/动作 短语</code>，会回复“你 动作 某个人 短语！”"),
    })),
    task: rong,
};
