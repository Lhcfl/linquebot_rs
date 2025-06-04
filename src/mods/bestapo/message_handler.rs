use super::toggle::BestapoCensor;
use super::utils::{is_contains_url, is_zero_width_char};
use crate::linquebot::{msg_context::Context, types::Consumption, Module};
use crate::utils::telegram::prelude::{MessageExtension, WarnOnError};
use log::{debug, warn};
use std::time::Duration;
use teloxide_core::{
    prelude::{Request, Requester},
    types::Message,
};
use tokio::time::sleep;

const SENSITIVE_WORDS: &[&str] = &["trump", "nft", "opensea"];

fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let ctx = ctx.task();
    if !msg.is_reply_to_channel() || !is_contains_url(msg) {
        return Consumption::just_next();
    }
    let mut is_spam = false;
    if let Some(text) = msg.text() {
        let text = text.to_lowercase().replace(is_zero_width_char, "");
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
    Consumption::next_with(async move {
        let enabled = ctx
            .app
            .db
            .of::<BestapoCensor>()
            .chat(ctx.chat_id)
            .get_or_insert(BestapoCensor::default)
            .await
            .censor_enabled;
        if !enabled {
            return;
        }
        ctx.reply_markdown("检测到斯帕姆。把它上市！")
            .send()
            .warn_on_error("bestapo")
            .await;
        sleep(Duration::from_secs(3)).await;
        ctx.app
            .bot
            .delete_message(ctx.chat_id, ctx.message_id)
            .send()
            .warn_on_error("bestapo")
            .await;
    })
}

pub static MESSAGE_HANDLER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
