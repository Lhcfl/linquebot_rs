use serde::{Deserialize, Serialize};
use teloxide_core::{prelude::Request, types::Message};

use crate::{
    linquebot::{
        msg_context::Context, types::Consumption, Module, ModuleDescription, ModuleKind,
    },
    utils::telegram::prelude::WarnOnError,
};

#[derive(Debug, Serialize, Deserialize)]
struct BestapoCensor {
    censor_enabled: bool,
}

fn on_toggle(ctx: &mut Context, _: &Message) -> Consumption {
    let ctx = ctx.task();
    async move {
        let mut stat = ctx
            .app
            .db
            .of::<BestapoCensor>()
            .chat(ctx.chat_id)
            .get_or_insert(|| BestapoCensor {
                censor_enabled: false,
            })
            .await;

        stat.censor_enabled = !stat.censor_enabled;

        if stat.censor_enabled {
            ctx.reply("北世太保审查已打开")
        } else {
            ctx.reply("北世太保审查已关闭")
        }
        .send()
        .warn_on_error("toggle_bestapo")
        .await;
    }
    .into()
}

pub static TOGGLE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "toggle_bestapo",
        description: "打开/关闭<b>北世太保</b>模块的频道回复审查功能",
        description_detailed: Some(concat!(
            "该命令不需要参数。\n",
            "打开/关闭<b>北世太保</b>模块的频道回复审查功能。\n",
            "北世太保会帮助你管理群组：审查伪人的信息，并把他们送去见主席。"
        )),
    }),
    task: on_toggle,
};
