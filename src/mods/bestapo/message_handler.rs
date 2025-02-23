use std::time::Duration;

use crate::{mods::bestapo::utils::is_contains_url, utils::telegram::prelude::MessageExtension};
use log::{debug, warn};
use teloxide_core::{
    prelude::{Request, Requester},
    types::Message,
};
use tokio::time::sleep;

use crate::{
    linquebot::{msg_context::Context, types::Consumption, Module},
    utils::telegram::prelude::WarnOnError,
};

const SENSITIVE_WORDS: &[&str] = &["trump", "nft", "opensea"];

fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let ctx = ctx.task();
    if !msg.is_reply_to_channel() || !is_contains_url(msg) {
        return Consumption::just_next();
    }
    let mut is_spam = false;
    if let Some(text) = msg.text() {
        let text = text.to_lowercase();
        for word in SENSITIVE_WORDS.iter() {
            if text.contains(word) {
                warn!("Sensitive word detected: {}", word);
                debug!("Message: {:#?}", msg);
                is_spam = true;
                break;
            }
        }
    }
    if !is_spam {
        return Consumption::just_next();
    }
    tokio::spawn(async move {
        ctx.reply_markdown("检测到斯帕姆。把它上市！")
            .send()
            .warn_on_error("bestapo")
            .await;
        sleep(Duration::from_secs(3)).await;
        ctx.app
            .bot
            .delete_message(ctx.chat_id, ctx.message_id)
            .send()
            .await
    });
    Consumption::just_next()
}

pub static MESSAGE_HANDLER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
