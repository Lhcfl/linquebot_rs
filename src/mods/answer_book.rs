/// 答案之书命令
use log::warn;
use msg_context::Context;
use rand::seq::SliceRandom;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::answer_book;
use crate::linquebot::*;
use crate::Consumption;

fn on_message(ctx: &mut Context, _message: &Message) -> Consumption {
    let ctx = ctx.task();
    Consumption::StopWith(Box::pin(async move {
        let chosen = answer_book::ANSWERS
            .choose(&mut rand::thread_rng())
            .expect("not empty");
        let res = ctx.reply(*chosen).send().await;
        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "answer",
        description: "答案之书",
        description_detailed: Some(concat!(
            "该命令不需要参数。\n",
            "调用《答案之书》给出<del>显然一点用也没有的</del>回答。"
        )),
    }),
    task: on_message,
};
