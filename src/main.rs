#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(str_split_whitespace_remainder)]
#![feature(iter_array_chunks)]
#![feature(iter_intersperse)]
#![feature(trait_upcasting)]
#![feature(async_drop)]
#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(stmt_expr_attributes)]
#![feature(iter_from_coroutine)]
#![feature(extend_one)]
#![feature(iter_next_chunk)]
#![feature(vec_pop_if)]

mod assets;
mod globals;
mod linquebot;
mod mods;
mod resolvers;
mod test_utils;
mod utils;

use crate::db::DataStorage;
use crate::linquebot::types::*;
use crate::linquebot::*;
use colored::Colorize;
use env_logger::Env;
use log::{error, info, warn};
use std::sync::OnceLock;
use teloxide_core::prelude::*;
use teloxide_core::types::BotCommand;
use teloxide_core::types::True;
use teloxide_core::RequestError;

static APP: OnceLock<App> = OnceLock::new();

async fn set_my_commands(app: &'static App) -> Result<True, RequestError> {
    let commands = app
        .modules
        .iter()
        .filter_map(|module| {
            if let ModuleKind::Command(cmd) = &module.kind {
                Some(BotCommand::new(cmd.name, cmd.description))
            } else {
                None
            }
        });

    app.bot.set_my_commands(commands).send().await
}

async fn init_app() -> anyhow::Result<&'static linquebot::App> {
    info!(target: "init", "Loading Database...");
    let db = DataStorage::new().await?;
    info!(target: "init", "Initializing Bot...");
    let bot = Bot::from_env();
    info!(target: "init", "Checking Network...");
    let me = bot.get_me().await?;
    info!(target: "init", "user id: {}", me.id);
    let _ = APP.set(linquebot::App {
        name: "琳酱".to_string(),
        username: me.username().to_string(),
        bot,
        db,
        modules: mods::MODULES,
        micro_tasks: mods::MICRO_TASKS,
    });
    let app = APP.get().expect("should initialized app");
    info!(target: "init", "user name: {}", app.username);
    info!(target: "init", "Settiing commands...");
    set_my_commands(app).await?;
    info!(target: "init", "{}", "Successfully initialized bot".green());
    Ok(app)
}

async fn main_loop() -> anyhow::Result<()> {
    let app = init_app().await?;
    let bot = &app.bot;

    let mut offset: i32 = 0;

    loop {
        match bot.get_updates().offset(offset).timeout(10).send().await {
            Ok(updates) => {
                offset = updates
                    .last().map(|u| u.id.0 as i32 + 1)
                    .unwrap_or(offset);

                for update in updates {
                    resolvers::update::resolve(app, update).await;
                }
            }
            Err(err) => {
                warn!(target: "main-loop", "GetUpdate Error: {}", err.to_string());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    if let Err(err) = main_loop().await {
        error!("main-loop panicked: {}", err.to_string());
        panic!("main-loop panicked: {}", err);
    }
    println!("bye bye");
}
