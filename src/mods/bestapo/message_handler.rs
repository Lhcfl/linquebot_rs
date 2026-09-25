use super::rules::matching_rule;
use super::toggle::BestapoCensor;
use crate::linquebot::{Module, msg_context::Context, types::Consumption};
use crate::utils::telegram::prelude::WarnOnError;
use log::{debug, warn};
use std::time::Duration;
use teloxide_core::{
    prelude::{Request, Requester},
    types::Message,
};
use tokio::time::sleep;

fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let Some(rule) = matching_rule(msg) else {
        return Consumption::just_next();
    };
    warn!("Spam rule matched: {}", rule);
    debug!("Message: {:#?}", msg);
    let ctx = ctx.task();
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
