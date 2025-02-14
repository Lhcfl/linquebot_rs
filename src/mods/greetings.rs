use std::{collections::HashMap, time::SystemTime};

use crate::{
    linquebot::{
        msg_context::Context,
        types::Consumption,
        Module, ModuleDescription, ModuleKind,
    },
    utils::telegram::prelude::WarnOnError,
};
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use teloxide_core::{
    prelude::Request,
    types::{Message, UserId},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GreetingStat {
    enabled: bool,
    last_3_msg_date: HashMap<UserId, [SystemTime; 3]>,
}

impl Default for GreetingStat {
    fn default() -> Self {
        GreetingStat {
            enabled: true,
            last_3_msg_date: HashMap::new(),
        }
    }
}

fn toggle_greeting(ctx: &mut Context, _msg: &Message) -> Consumption {
    let ctx = ctx.task();
    async move {
        let mut db = ctx
            .app
            .db
            .of::<GreetingStat>()
            .chat(ctx.chat_id)
            .get_or_insert(Default::default)
            .await;
        db.enabled = !db.enabled;
        let text = if db.enabled { "打开" } else { "关闭" };
        ctx.reply(format!("本群的打招呼功能已{text}"))
            .send()
            .warn_on_error("greeting")
            .await;
    }
    .into()
}

mod morning {
    use crate::{
        linquebot::{msg_context::TaskContext, TaskResult},
        utils::telegram::prelude::*,
    };
    use teloxide_core::{prelude::*, types::User};

    fn morning_1(ctx: TaskContext, _: User) -> TaskResult {
        Box::pin(ctx.reply("早安喵").send().warn_on_error("greeting"))
    }
    fn morning_2(ctx: TaskContext, user: User) -> TaskResult {
        Box::pin(
            ctx.reply(format!(
                "早安, {}~ 新的一天也会有新的美好的~",
                user.full_name()
            ))
            .send()
            .warn_on_error("greeting"),
        )
    }
    fn morning_3(ctx: TaskContext, _: User) -> TaskResult {
        Box::pin(
            ctx.app
                .bot
                .send_sticker_by_file_id(ctx.chat_id, "CAACAgUAAx0CYVtTrQABBnNDZ679Ld8mNB2n3255MtxXjUPeObUAAkwGAAJYxiBXkpKDglGMbcc2BA")
                .warn_on_error("greeting"),
        )
    }

    pub static MORNING: [fn(TaskContext, User) -> TaskResult; 3] =
        [morning_1, morning_2, morning_3];
}

#[derive(Debug, Clone, Copy)]
enum GreetingKind {
    None,
    Morning,
    Night,
}

fn say_greeting(ctx: &mut Context, msg: &Message) -> Consumption {
    let text = msg.text()?;
    let mut greeting = GreetingKind::None;
    if text.len() < 50 && text.contains("早安") {
        greeting = GreetingKind::Morning;
    }
    if text.len() < 50 && text.contains("晚安") {
        greeting = GreetingKind::Night;
    }
    let force = text.contains(&format!("@{}", ctx.app.username));
    let ctx = ctx.task();
    let user = msg.from.as_ref()?.clone();
    let is_reply = msg.reply_to_message().is_some();
    tokio::spawn(async move {
        let mut db = ctx
            .app
            .db
            .of::<GreetingStat>()
            .chat(ctx.chat_id)
            .get_or_insert(Default::default)
            .await;

        if !db.enabled {
            return;
        }

        let last3 = db
            .last_3_msg_date
            .entry(user.id)
            .or_insert([SystemTime::UNIX_EPOCH; 3]);

        let now = SystemTime::now();

        match greeting {
            GreetingKind::None => {}
            GreetingKind::Morning => {
                // 如果3句前说话了 不早安
                // 如果是回复别人的 不早安
                if (!is_reply
                    && now
                        .duration_since(*last3.last().unwrap())
                        .is_ok_and(|dur| dur.as_secs() > 3600))
                    || force
                {
                    tokio::spawn(morning::MORNING
                        .iter()
                        .choose(&mut rand::thread_rng())
                        .unwrap()(ctx, user));
                }
            }
            GreetingKind::Night => {
                if force || !is_reply {
                    ctx.reply("晚安喵").send().warn_on_error("greeting").await;
                }
            }
        }

        last3.rotate_right(1);
        last3[0] = now;
    });
    Consumption::Next
}

pub static MODULE: Module = Module {
    kind: ModuleKind::General(None),
    task: say_greeting,
};

pub static TOGGLE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "toggle_greeting",
        description: "打开/关闭打招呼",
        description_detailed: None,
    }),
    task: toggle_greeting,
};
