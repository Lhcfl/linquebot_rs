use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::Message;

use crate::msg_context::Context;
use crate::{Consumption, Module, ModuleDesctiption, ModuleKind};

fn send_help(ctx: &mut Context, _msg: &Message) -> Consumption {
    let ctx = ctx.task();
    Consumption::StopWith(Box::pin(async move {
        let mut command_texts = Vec::<String>::new();
        let mut general_texts = Vec::<String>::new();
        for module in ctx.app.modules {
            match &module.kind {
                ModuleKind::Command(cmd) => {
                    command_texts.push(format!("/{}: {}", cmd.name, cmd.description));
                    if let Some(long_desc) = cmd.description_detailed {
                        command_texts.push(long_desc.to_string());
                    }
                }
                ModuleKind::General(Some(cmd)) => {
                    general_texts.push(format!("**{}**: {}", cmd.name, cmd.description,));
                    if let Some(long_desc) = cmd.description_detailed {
                        general_texts.push(long_desc.to_string());
                    }
                }
                _ => {}
            }
        }

        let res = ctx
            .reply_markdown(&format!(
                "OoO 这里是琳酱帮助:\n{}\n{}",
                command_texts.join("\n"),
                general_texts.join("\n")
            ))
            .send()
            .await;

        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "help",
        description: "显示帮助",
        description_detailed: None,
    }),
    task: send_help,
};
