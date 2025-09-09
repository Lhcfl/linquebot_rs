//! jielong  
//! 成语接龙
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;

use log::error;
use log::warn;
use msg_context::Context;
use msg_context::TaskContext;
use rand::thread_rng;
use rand::Rng;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::idiom::*;
use crate::linquebot::*;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

/// 接龙用户
struct JielongUser {
    max_combo: u32,
    score: u32,
    pretty_name: String,
}

// 接龙
struct Jielong {
    /** 接龙 id */
    nonce: u64,
    /** 上一个被接龙的成语 */
    idiom: &'static Idiom,
    /** 已经被接龙的词语集合 */
    counted: HashSet<String>,
    /** 上一个接龙的userid */
    last_jielong_user: UserId,
    /** 目前连击数 */
    combo: u32,
    /** 用户得分信息 */
    users: HashMap<UserId, JielongUser>,
    start_at: SystemTime,
    /** 储存 Task */
    ctx: TaskContext,
}

impl Jielong {
    fn new_with(ctx: TaskContext, idiom: &'static Idiom) -> Jielong {
        Jielong {
            nonce: thread_rng().gen(),
            idiom,
            counted: HashSet::new(),
            last_jielong_user: UserId(0),
            combo: 0,
            users: HashMap::new(),
            start_at: SystemTime::now(),
            ctx,
        }
    }
    fn new(ctx: TaskContext) -> Jielong {
        Jielong::new_with(ctx, random_idiom())
    }
    fn update_users(&mut self, current_user: &User) {
        if let Some(old_user) = self.users.get_mut(&current_user.id) {
            old_user.score += match self.combo {
                1..3 => 1,
                3..6 => 2,
                6..10 => 3,
                _ => 4,
            };
            old_user.max_combo = self.combo.max(old_user.max_combo);
        } else {
            self.users.insert(
                current_user.id,
                JielongUser {
                    max_combo: 1,
                    score: 1,
                    pretty_name: current_user.mention().unwrap_or(current_user.full_name()),
                },
            );
        }
    }
    fn pretty_result(&self) -> String {
        let mut users = self.users.values().collect::<Vec<_>>();
        users.sort_by(|a, b| b.score.cmp(&a.score));
        users
            .into_iter()
            .enumerate()
            .map(|(index, user)| {
                let index = index + 1;
                format!(
                    "第 {index} 名: {} {} 分，最大连击 {} 次",
                    user.pretty_name, user.score, user.max_combo
                )
            })
            .intersperse("\n".to_string())
            .collect()
    }
}

static CHAT_JIELONG_STATUS: LazyLock<RwLock<HashMap<ChatId, Jielong>>> =
    LazyLock::new(Default::default);

static HELP_MESSAGE: &str = concat!(
    "需要至少一个参数\n",
    "<code>/jielong start</code>: 开始成语接龙\n",
    "<code>/jielong stop</code>: 强制停止成语接龙\n",
    "<code>/jielong show</code>: 显示当前接龙信息\n",
    "<code>/jielong [成语]</code>: 以你提供的成语开始成语接龙\n",
);

fn stop_jielong(chat_id: ChatId, nonce: u64) {
    let Ok(mut status) = CHAT_JIELONG_STATUS.write() else {
        error!("Failed to read CHAT_JIELONG_STATUS");
        return;
    };
    if let Some(jielong) = status.get(&chat_id) {
        if jielong.nonce != nonce {
            return;
        }
    }
    if let Some(jielong) = status.remove(&chat_id) {
        tokio::spawn(async move {
            let _ = jielong
                .ctx
                .app
                .bot
                .send_message(
                    chat_id,
                    format!(
                        "成语接龙结束啦！琳酱来宣布结果：\n\n{}",
                        jielong.pretty_result()
                    ),
                )
                .send()
                .await;
        });
    }
}

fn try_stop_jielong_with(ctx: TaskContext, from: &User) -> Consumption {
    let Ok(mut status) = CHAT_JIELONG_STATUS.write() else {
        error!("Failed to read CHAT_JIELONG_STATUS");
        return ctx
            .reply("发生了未知错误……")
            .send()
            .warn_on_error("jielong-report-error")
            .into();
    };
    if let Some(jielong) = status.remove(&ctx.chat_id) {
        ctx.reply(format!(
            "成语接龙被 {} 结束啦！琳酱来宣布结果：\n\n{}",
            from.full_name(),
            jielong.pretty_result()
        ))
        .send()
        .warn_on_error("stop-jielong-manual")
        .into()
    } else {
        ctx.reply("接龙还没开始哦。")
            .send()
            .warn_on_error("stop-jielong-manual")
            .into()
    }
}

fn try_start_jielong_with(ctx: TaskContext, init: &str) -> Consumption {
    let Ok(mut status) = CHAT_JIELONG_STATUS.write() else {
        error!("Failed to read CHAT_JIELONG_STATUS");
        return ctx
            .reply("发生了未知错误……")
            .send()
            .warn_on_error("jielong-report-error")
            .into();
    };
    if status.get(&ctx.chat_id).is_some() {
        return ctx
            .reply("已经有一个接龙正在进行中啦！")
            .send()
            .warn_on_error("start-jielong")
            .into();
    }
    let task = ctx.clone();
    let jielong = if init.is_empty() {
        Jielong::new(ctx)
    } else {
        let Some(idiom) = get_idiom(init) else {
            return ctx
                .reply("没有这个成语哦")
                .send()
                .warn_on_error("start-jielong")
                .into();
        };
        Jielong::new_with(ctx, idiom)
    };
    let current = jielong.idiom;
    let nonce = jielong.nonce;
    status.insert(jielong.ctx.chat_id, jielong);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_mins(10)).await;
        stop_jielong(task.chat_id, nonce);
    });

    let hint = if init.is_empty() {
        "琳酱来给出第一个成语吧"
    } else {
        "第一个成语是"
    };

    task.reply(format!(
        "开始接龙！{hint}：{}, 请接 {}",
        current.word, current.last
    ))
    .send()
    .warn_on_error("start-jielong")
    .into()
}

fn show_jielong_status(ctx: TaskContext) -> Consumption {
    let Ok(status) = CHAT_JIELONG_STATUS.read() else {
        error!("Failed to read CHAT_JIELONG_STATUS");
        return ctx
            .reply("发生了未知错误……")
            .send()
            .warn_on_error("jielong-report-error")
            .into();
    };
    if let Some(jielong) = status.get(&ctx.chat_id) {
        ctx.reply(format!(
            "接龙游戏开始于：{} \n当前成语：{}，请接：{}\n\n{}",
            match SystemTime::now().duration_since(jielong.start_at) {
                Ok(time) => format!("{} 分钟 {} 秒前", time.as_secs() / 60, time.as_secs() % 60),
                Err(_) => "某个奇怪的时间裂隙前".to_string(),
            },
            jielong.idiom.word,
            jielong.idiom.last,
            jielong.pretty_result()
        ))
        .send()
        .warn_on_error("show-jielong-status")
        .into()
    } else {
        ctx.reply("没有正在进行的接龙游戏哦")
            .send()
            .warn_on_error("show-jielong-status")
            .into()
    }
}

fn on_jielong_command(ctx: &mut Context, message: &Message) -> Consumption {
    let args = ctx.cmd?.content;
    let ctx = ctx.task();
    let from = message.from.as_ref()?;
    match args {
        "stop" => try_stop_jielong_with(ctx, from),
        "start" => try_start_jielong_with(ctx, ""),
        "show" => show_jielong_status(ctx),
        "" => async move {
            if let Err(err) = ctx.reply_html(HELP_MESSAGE).send().await {
                warn!("Failed to send reply: {}", err);
            }
        }
        .into(),
        x => try_start_jielong_with(ctx, x),
    }
}

fn on_jielong_message(ctx: &mut Context, message: &Message) -> Consumption {
    let from = message.from.as_ref()?;
    let msg_idiom: &'static Idiom = get_idiom(message.text()?)?;
    let Ok(mut status) = CHAT_JIELONG_STATUS.write() else {
        error!("Failed to read CHAT_JIELONG_STATUS");
        return Consumption::just_next();
    };
    let Some(record) = status.get_mut(&ctx.chat_id) else {
        return Consumption::just_next();
    };
    if msg_idiom.first != record.idiom.last {
        return Consumption::just_next();
    }
    let ctx = ctx.task();
    if record.counted.contains(&msg_idiom.word) {
        ctx.reply("这个成语接过了哦")
            .send()
            .warn_on_error("check-jielong")
            .into()
    } else {
        let combo = if from.id == record.last_jielong_user {
            record.combo + 1
        } else {
            1
        };
        record.combo = combo;
        record.counted.insert(msg_idiom.word.clone());
        record.last_jielong_user = from.id;
        record.idiom = msg_idiom;
        record.update_users(from);

        let full_name = from.full_name();
        if combo >= 3 {
            ctx.app
                .bot
                .send_message(
                    ctx.chat_id,
                    format!("{full_name} {combo} 连击！下一个: {}", msg_idiom.last),
                )
                .send()
                .warn_on_error("check-jielong")
                .into()
        } else {
            ctx.app
                .bot
                .send_message(
                    ctx.chat_id,
                    format!("接龙成功！{full_name} 分数+1。下一个: {}", msg_idiom.last),
                )
                .send()
                .warn_on_error("check-jielong")
                .into()
        }
    }
}

pub static COMMAND: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "jielong",
        description: "成语接龙",
        description_detailed: Some(HELP_MESSAGE),
    }),
    task: on_jielong_command,
};

pub static ON_IDIOM: Module = Module {
    kind: ModuleKind::General(None),
    task: on_jielong_message,
};
