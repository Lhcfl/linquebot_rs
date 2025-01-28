//! 只是一个简单的复述句子的模块

use msg_context::Context;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;

pub fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let text = ctx.cmd?.content;
    let mut reply = ctx.app.bot.send_message(ctx.chat_id, text);
    if let Some(reply_to_message) = msg.reply_to_message() {
        reply = reply.reply_parameters(ReplyParameters::new(reply_to_message.id));
    }
    Consumption::StopWith(Box::pin(reply.send().warn_on_error("say")))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "say",
        description: "复述内容",
        description_detailed: None,
    }),
    task: on_message,
};
