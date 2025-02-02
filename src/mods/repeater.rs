//! 复读机

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;

use msg_context::Context;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;

struct MessageHistory {
    text: Option<String>,
    repeated: u32,
    off: bool,
}

static LAST_MSG: LazyLock<RwLock<HashMap<ChatId, MessageHistory>>> =
    LazyLock::new(Default::default);

/// repeat the message if 3 continous same text
pub fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let text = msg.text().map(|str| str.to_string());
    let mut manager = LAST_MSG.write().map_err(|err| {
        log::error!(
            "Error get history lock. This is not expected. {}",
            err.to_string()
        );
    })?;
    let Some(history) = manager.get_mut(&ctx.chat_id) else {
        manager.insert(
            ctx.chat_id,
            MessageHistory {
                text,
                repeated: 1,
                off: false,
            },
        );
        return Consumption::Next;
    };
    if history.off {
        return Consumption::Next;
    }
    if text == history.text {
        history.repeated += 1;
    } else {
        history.repeated = 1;
        history.text = text.clone();
    }
    let text = text?;

    // only repeat once!
    if history.repeated == 3
    // don't repeat command
    && !text.starts_with("/")
    {
        tokio::spawn(
            ctx.app
                .bot
                .send_message(ctx.chat_id, text)
                .send()
                .warn_on_error("repeater"),
        );
    }
    Consumption::Next
}

pub fn toggle_repeat(ctx: &mut Context, _: &Message) -> Consumption {
    let mut manager = LAST_MSG.write().map_err(|err| {
        log::error!(
            "Error get history lock. This is not expected. {}",
            err.to_string()
        );
    })?;
    let Some(history) = manager.get_mut(&ctx.chat_id) else {
        manager.insert(
            ctx.chat_id,
            MessageHistory {
                text: None,
                repeated: 0,
                off: true,
            },
        );
        return Consumption::Next;
    };
    *history = MessageHistory {
        text: None,
        repeated: 0,
        off: !history.off,
    };
    let text = if history.off {
        "复读姬已关闭"
    } else {
        "复读姬已打开"
    };
    Consumption::StopWith(Box::pin(
        ctx.task().reply(text).send().warn_on_error("toggle-repeat"),
    ))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::General(None),
    task: on_message,
};

pub static TOGGLE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "toggle_repeat",
        description: "开关复读姬",
        description_detailed: Some(concat!(
            "默认打开复读。\n",
            "不接受任何参数，每次收到这个命令时，如果复读姬是关的，就打开它，反之则关闭它。"
        )),
    }),
    task: toggle_repeat,
};
