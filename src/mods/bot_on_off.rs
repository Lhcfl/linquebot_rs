//! bot的开关
//! 默认开启，无持久化

use log::error;
use log::warn;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::msg_context::Context;
use crate::Consumption;
use crate::Module;
use crate::ModuleDescription;
use crate::ModuleKind;

static BOT_ON: LazyLock<RwLock<HashMap<ChatId, bool>>> = LazyLock::new(Default::default);

fn on_bot_on_message(ctx: &mut Context, _: &Message) -> Consumption {
    let Ok(mut record) = BOT_ON.write() else {
        error!("Failed to get bot on status!");
        return Consumption::just_stop();
    };

    let ctx = ctx.task();
    if record.remove(&ctx.chat_id).is_some() {
        async move {
            if let Err(err) = ctx.reply("琳酱已开机").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }
        .into()
    } else {
        async move {
            if let Err(err) = ctx.reply("琳酱处于开机状态").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }
        .into()
    }
}
fn on_bot_off_message(ctx: &mut Context, _: &Message) -> Consumption {
    let Ok(mut record) = BOT_ON.write() else {
        error!("Failed to get bot on status!");
        return Consumption::just_stop();
    };

    let ctx = ctx.task();
    if let Some(false) = record.insert(ctx.chat_id, false) {
        async move {
            if let Err(err) = ctx.reply("琳酱处于关机状态").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }
        .into()
    } else {
        async move {
            if let Err(err) = ctx.reply("琳酱已关机").send().await {
                warn!("Failed to send reply: {}", err.to_string());
            }
        }
        .into()
    }
}

fn stop_when_bot_off(ctx: &mut Context, _: &Message) -> Consumption {
    if let Ok(record) = BOT_ON.read() {
        if let Some(false) = record.get(&ctx.chat_id) {
            return Consumption::just_stop();
        }
    }
    Consumption::just_next()
}

pub static BOT_ON_MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "bot_on",
        description: "打开 bot",
        description_detailed: None,
    }),
    task: on_bot_on_message,
};
pub static BOT_OFF_MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "bot_off",
        description: "关闭 bot",
        description_detailed: None,
    }),
    task: on_bot_off_message,
};
pub static STOP_WHEN_BOT_OFF: Module = Module {
    kind: ModuleKind::General(None),
    task: stop_when_bot_off,
};
