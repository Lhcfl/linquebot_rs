use log::warn;
use msg_context::Context;
use msg_context::TaskContext;
use rand::seq::IteratorRandom;
use rand::Rng;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;

async fn send_raw_rand(ctx: TaskContext, from: User, text_body: String) {
    let result = rand::thread_rng().gen_range(0..=100);
    let msg = format!(
        "{} {}",
        from.html_link(),
        if text_body.trim().is_empty() {
            format!("掷出了: {result}")
        } else {
            format!("{} 的概率是: {result}%", escape_html(text_body.trim()))
        }
    );
    if let Err(err) = ctx.reply_html(&msg).send().await {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

async fn send_selective_rand(ctx: TaskContext, text_body: String, spliter: &str) {
    let result = text_body
        .split(&spliter)
        .choose(&mut rand::thread_rng())
        .unwrap_or("undefined");

    if let Err(err) = ctx
        .reply_html(&format!("{}!", escape_html(result)))
        .send()
        .await
    {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

pub fn on_message(ctx: &mut Context, message: &Message) -> Consumption {
    let text = ctx.cmd?.content.to_string();
    let Some(from) = message.from.clone() else {
        warn!("No reply target.");
        return Consumption::Stop;
    };
    if text.contains("还是") {
        Consumption::StopWith(Box::pin(send_selective_rand(ctx.task(), text, "还是")))
    } else {
        Consumption::StopWith(Box::pin(send_raw_rand(ctx.task(), from, text)))
    }
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "rand",
        description: "随机一下",
        description_detailed: Some(concat!(
            "可选参数：事件\n",
            "当发送 <code>/rand 事件</code> 的时候，会回复“事件 的概率是 {rand}%”\n",
            "如果事件被省略，则会回复“掷出了 {rand}”\n\n",
            "特殊关键字：当发送 <code>/rand A还是B</code> 的时候，会在这些“还是”中选择一个发送回来\n",
        )),
    }),
    task: on_message,
};
