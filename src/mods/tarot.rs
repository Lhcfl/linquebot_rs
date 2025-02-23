/// 抽取塔罗牌
use std::time::Duration;

use log::warn;
use msg_context::Context;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::tarot;
use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;

pub fn on_message(ctx: &mut Context, message: &Message) -> Consumption {
    let text = ctx.cmd?.content;
    let Some(from) = message.from.clone() else {
        warn!("No reply target.");
        return Consumption::just_stop();
    };
    let num;

    let ctx = ctx.task();

    if text.is_empty() {
        num = 3;
    } else if let Ok(parsed) = text.parse::<usize>() {
        num = parsed;
    } else {
        return ctx
            .reply("数字不对，不准乱玩琳酱呀")
            .send()
            .warn_on_error("tarot")
            .into();
    };
    if num == 0 {
        return ctx
            .reply("不给你牌可以，可以给你一拳")
            .send()
            .warn_on_error("tarot")
            .into();
    }
    if num > 21 {
        return ctx
            .reply("牌都给你摸完了，不准乱玩琳酱")
            .send()
            .warn_on_error("tarot")
            .into();
    }

    async move {
        ctx.reply(format!(
            "{}最近遇到了什么烦心事吗？让琳酱给你算一算:",
            from.full_name()
        ))
        .send()
        .warn_on_error("tarot")
        .await;

        ctx.app
            .bot
            .send_chat_action(ctx.chat_id, ChatAction::Typing)
            .send()
            .warn_on_error("tarot")
            .await;

        tokio::time::sleep(Duration::from_millis(2500)).await;

        let text = tarot::n_random_majors(num)
            .into_iter()
            .map(|tarot| tarot.to_string())
            .collect::<Vec<_>>();

        ctx.reply(format!(
            "{} 抽到的牌组是: \n{}",
            from.full_name(),
            text.join("\n")
        ))
        .send()
        .warn_on_error("tarot")
        .await;
    }
    .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "tarot",
        description: "抽取塔罗牌",
        description_detailed: Some(concat!("可选参数：数量\n", "默认摸 3 张。\n",)),
    }),
    task: on_message,
};
