#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]

mod assets;
mod linquebot;
mod mods;
mod utils;

use crate::linquebot::types::*;
use chrono::Utc;
use log::{error, info, trace, warn};
use simple_logger::SimpleLogger;
use std::sync::OnceLock;
use teloxide_core::{
    prelude::*,
    types::{Message, Update, UpdateKind},
    RequestError,
};

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
static MODULE_HANDLES: &[fn(&Bot, &Message) -> Option<ComsumedType>] = &[
    mods::bot_on_off::on_message,
    mods::rand::on_message,
    mods::set_title::on_message,
    mods::todo::on_message,
    mods::hitokoto::on_message,
    mods::answer_book::on_message,
    mods::rong::on_message,
];

pub static BOT_USERNAME: OnceLock<String> = OnceLock::new();

fn module_resolver(bot: &Bot, message: &Message) -> () {
    trace!(target: "main-loop", "get message: {:?}", message.text());

    for handle in MODULE_HANDLES {
        if let Some(ComsumedType::Stop) = handle(bot, message) {
            break;
        }
    }
}

async fn update_resolver(bot: &Bot, update: Update) {
    let now = Utc::now();
    if let UpdateKind::Message(message) = update.kind {
        if now.signed_duration_since(&message.date).num_seconds() > 30 {
            warn!(
                target: "main-loop",
                "skipped message {}s ago: {:?}",
                now.signed_duration_since(&message.date).num_seconds(),
                message.text()
            );
            return;
        }
        module_resolver(bot, &message);
    }
}

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

    let mut offset: i32 = 0;

    loop {
        match bot.get_updates().offset(offset).timeout(10).send().await {
            Ok(updates) => {
                offset = updates
                    .last()
                    .and_then(|u| Some(u.id.0 as i32 + 1))
                    .unwrap_or(offset);

                for update in updates {
                    update_resolver(&bot, update).await;
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
