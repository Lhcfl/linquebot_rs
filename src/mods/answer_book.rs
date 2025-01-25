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
        let res = ctx.reply(*&chosen).send().await;
        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "answer",
        description: "答案之书",
        description_detailed: None,
    }),
    task: on_message,
};
