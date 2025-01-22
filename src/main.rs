#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]

mod assets;
mod globals;
mod linquebot;
mod mods;
mod test_utils;
mod utils;

use std::sync::OnceLock;

use crate::linquebot::types::*;
use chrono::Utc;
use linquebot::*;
use log::{error, info, trace, warn};
use simple_logger::SimpleLogger;
use teloxide_core::{
    prelude::*,
    types::{Message, Update, UpdateKind},
    RequestError,
};

/// Module Handles 的顺序很重要
/// 请确保这些函数是拓扑排序的
// static MODULE_HANDLES: &[fn(&Bot, &Message) -> Consumption] = &[
//     mods::skip_other_bot::on_message,
//     mods::bot_on_off::on_message,
//     mods::rand::on_message,
//     mods::set_title::on_message,
//     mods::todo::on_message,
//     mods::hitokoto::on_message,
//     mods::answer_book::on_message,
//     mods::rong::on_message,
// ];
static APP: OnceLock<App> = OnceLock::new();

fn module_resolver(app: &'static App, message: Message) -> () {
    trace!(target: "main-loop", "get message: {:?}", message.text());
    for module in app.modules {
        let task_result = (module.task)(app, &message);
        match task_result {
            Consumption::Next => {}
            Consumption::Stop => {
                break;
            }
            Consumption::StopWith(task) => {
                tokio::spawn(async move {
                    task.await;
                });
                break;
            }
        }
    }
}

async fn update_resolver(app: &'static App, update: Update) {
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
        module_resolver(app, message);
    }
}

async fn init_app() -> Result<&'static linquebot::App, RequestError> {
    info!(target: "main", "Initializing Bot...");
    let bot = Bot::from_env();
    info!(target: "main", "Checking Network...");
    let me = bot.get_me().await?;
    info!(target: "main", "user id: {}", me.id);
    let _ = APP.set(linquebot::App {
        name: "琳酱".to_string(),
        username: me.username().to_string(),
        bot,
        modules: mods::MODULES,
    });
    let app = APP.get().expect("should initialized app");
    info!(target: "main", "user name: {}", app.username);
    Ok(app)
}

async fn main_loop() -> Result<(), RequestError> {
    let app = init_app().await?;
    let bot = &app.bot;

    let mut offset: i32 = 0;

    loop {
        match bot.get_updates().offset(offset).timeout(10).send().await {
            Ok(updates) => {
                offset = updates
                    .last()
                    .and_then(|u| Some(u.id.0 as i32 + 1))
                    .unwrap_or(offset);

                for update in updates {
                    update_resolver(&app, update).await;
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
