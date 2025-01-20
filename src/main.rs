#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(stmt_expr_attributes)]
#![feature(coroutines)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(let_chains)]
#![feature(assert_matches)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]

mod assets;
mod globals;
mod linquebot;
mod mods;
#[cfg(test)]
mod test_utils;
mod utils;

use crate::linquebot::types::*;
use chrono::Utc;
use globals::BOT_USERNAME;
use linquebot::BotRegistry;
use log::{error, info, trace, warn};
use simple_logger::SimpleLogger;
use teloxide_core::{
    prelude::*,
    types::{Update, UpdateKind},
    RequestError,
};
use utils::ContextStorage;

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
static MODULE_HANDLES: &[&dyn BotRegistry] = {
    use mods::*;
    &[
        &skip_other_bot::SkipOtherBot,
        /*
        mods::skip_other_bot::on_message,
        mods::bot_on_off::on_message,
        mods::rand::on_message,
        mods::set_title::on_message,
        mods::todo::on_message,
        mods::hitokoto::on_message,
        mods::answer_book::on_message,
        mods::rong::on_message,
        */
    ]
};

async fn init_bot() -> Result<Bot, RequestError> {
    info!(target: "main", "Initializing Bot...");
    let bot = Bot::from_env();
    info!(target: "main", "Checking Network...");
    let me = bot.get_me().await?;
    info!(target: "main", "user id: {}", me.id);
    BOT_USERNAME
        .set(me.username().to_string())
        .expect("set bot username");
    info!(target: "main", "user name: {}", BOT_USERNAME.get().expect("has username"));

    Ok(bot)
}

async fn main_loop() -> Result<(), RequestError> {
    let bot = init_bot().await?;
    let storage = ContextStorage::new();

    let resolve_update = |update: &Update| {
        let now = Utc::now();
        let UpdateKind::Message(message) = &update.kind else {
            return;
        };
        if now.signed_duration_since(&message.date).num_seconds() > 30 {
            warn!(
                target: "main-loop",
                "skipped message {}s ago: {:?}",
                now.signed_duration_since(&message.date).num_seconds(),
                message.text()
            );
            return;
        }
        trace!(target: "main-loop", "get message: {:?}", message.text());

        let mut data = storage.clone().make_context(&message);
        for reg in MODULE_HANDLES {
            match reg.match_message(&bot, &message, &mut data) {
                ConsumeKind::Decline => {}
                ConsumeKind::Action(fut) => {
                    tokio::spawn(fut);
                }
                ConsumeKind::Consume => break,
            }
        }
    };

    let mut offset: i32 = 0;

    loop {
        match bot.get_updates().offset(offset).timeout(10).send().await {
            Ok(updates) => {
                updates.last().map(|u| offset = u.id.0 as i32 + 1);

                for update in updates {
                    resolve_update(&update);
                }
            }
            Err(err) => {
                warn!(target: "main-loop", "GetUpdate Error: {}", err.to_string());
            }
        }
    }
}

#[tokio::main]
async fn main() -> () {
    SimpleLogger::new().init().unwrap();
    if let Err(err) = main_loop().await {
        error!("main-loop panicked: {}", err.to_string());
        panic!("main-loop panicked: {}", err.to_string());
    }
    println!("bye bye");
}
