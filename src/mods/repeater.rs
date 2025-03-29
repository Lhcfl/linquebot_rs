//! 复读机
use crate::linquebot::msg_context::TaskContext;
use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;
use msg_context::Context;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

#[derive(PartialEq)]
enum MsgKind {
    Sticker(String),
    Text(String),
    Other,
}

impl MsgKind {
    fn from_msg(msg: &Message) -> Self {
        if let Some(text) = msg.text() {
            MsgKind::Text(text.to_string())
        } else if let Some(sticker) = msg.sticker() {
            MsgKind::Sticker(sticker.file.id.clone())
        } else {
            MsgKind::Other
        }
    }
    async fn send_by_ctx(self, ctx: TaskContext) {
        match self {
            Self::Text(text) => {
                // 彩蛋
                let egg = text == "没有" || text == "没有！";

                ctx.app
                    .bot
                    .send_message(ctx.chat_id, text)
                    .send()
                    .warn_on_error("repeater")
                    .await;

                if egg {
                    ctx.app
                        .bot
                        .send_message(ctx.chat_id, "通过！")
                        .send()
                        .warn_on_error("repeater")
                        .await;
                }
            }
            Self::Sticker(sticker) => {
                ctx.app
                    .bot
                    .send_sticker_by_file_id(ctx.chat_id, &sticker)
                    .warn_on_error("repeater")
                    .await;
            }
            Self::Other => {}
        }
    }
}

struct MessageHistory {
    kind: MsgKind,
    repeated: u32,
    off: bool,
}

impl Default for MessageHistory {
    fn default() -> Self {
        MessageHistory {
            kind: MsgKind::Other,
            repeated: 1,
            off: false,
        }
    }
}

static LAST_MSG: LazyLock<RwLock<HashMap<ChatId, MessageHistory>>> =
    LazyLock::new(Default::default);

/// repeat the message if 3 continous same text
pub fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let kind = MsgKind::from_msg(msg);
    let mut manager = LAST_MSG.write().map_err(|err| {
        log::error!(
            "Error get history lock. This is not expected. {}",
            err.to_string()
        );
    })?;
    let Some(history) = manager.get_mut(&ctx.chat_id) else {
        manager.insert(ctx.chat_id, Default::default());
        return Consumption::just_next();
    };
    if history.off {
        return Consumption::just_next();
    }

    if kind == history.kind {
        history.repeated += 1;
        if history.repeated == 2 {
            return Consumption::next_with(kind.send_by_ctx(ctx.task()));
        }
    } else {
        history.repeated = 1;
        history.kind = kind;
    }
    Consumption::just_next()
}

pub fn toggle_repeat(ctx: &mut Context, _: &Message) -> Consumption {
    let mut manager = LAST_MSG.write().map_err(|err| {
        log::error!(
            "Error get history lock. This is not expected. {}",
            err.to_string()
        );
    })?;
    let history = manager.entry(ctx.chat_id).or_default();
    *history = MessageHistory {
        kind: MsgKind::Other,
        repeated: 0,
        off: !history.off,
    };
    let text = if history.off {
        "复读姬已关闭"
    } else {
        "复读姬已打开"
    };

    ctx.task()
        .reply(text)
        .send()
        .warn_on_error("toggle-repeat")
        .into()
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
