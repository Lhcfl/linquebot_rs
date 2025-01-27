#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(str_split_whitespace_remainder)]
#![feature(iter_array_chunks)]
#![feature(iter_intersperse)]

mod assets;
mod db;
mod globals;
mod linquebot;
mod mods;
mod resolvers;
mod test_utils;
mod utils;

use std::sync::OnceLock;

use crate::linquebot::types::*;
use db::DataStorage;
use linquebot::*;
use log::{error, info, warn};
use simple_logger::SimpleLogger;
use teloxide_core::{prelude::*, RequestError};

static APP: OnceLock<App> = OnceLock::new();

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
        db: DataStorage {},
        modules: mods::MODULES,
        micro_tasks: mods::MICRO_TASKS,
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
                    resolvers::update::resolve(&app, update).await;
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
