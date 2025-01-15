#![feature(str_split_remainder)]
#![feature(duration_constructors)]
#![feature(try_blocks)]

mod assets;
mod linquebot;
mod mods;
mod utils;
use crate::linquebot::types::*;

use chrono::Utc;
use colored::Colorize;
use teloxide_core::{
    prelude::*,
    types::{Message, Update, UpdateKind},
    RequestError,
};

static MODULE_HANDLES: &[fn(&Bot, &Message) -> Option<ComsumedType>] = &[
    mods::rand::on_message,
    mods::set_title::on_message,
    mods::todo::on_message,
    mods::hitokoto::on_message,
    mods::answer_book::on_message,
    mods::rong::on_message,
];

fn module_resolver(bot: &Bot, message: &Message) -> () {
    println!("text: {:?}", message.text());

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
            println!(
                "{}: skipped a message {:?} ago",
                "warning".yellow(),
                now.signed_duration_since(&message.date)
            );
            return;
        }
        module_resolver(bot, &message);
    }
}

async fn main_loop() -> Result<(), RequestError> {
    println!("Initializing Bot...");
    let bot = Bot::from_env();
    println!("Checking Network...");
    let me = bot.get_me().await?;
    println!("user id: {}", me.id);
    println!("user name: {}", me.username.as_ref().unwrap());
    let mut offset: i32 = 0;

    loop {
        match bot.get_updates().offset(offset).timeout(10).send().await {
            Ok(updates) => {
                offset = updates
                    .last()
                    .and_then(|u| Some(u.id.0 as i32 + 1))
                    .unwrap_or(offset);

                for update in updates {
                    println!("got update: {}", update.id.0);
                    update_resolver(&bot, update).await;
                }
            }
            Err(err) => {
                println!("request error: {}", err.to_string());
            }
        }
    }
}

#[tokio::main]
async fn main() -> () {
    if let Err(err) = main_loop().await {
        panic!("Oops! main loop error: {}", err.to_string());
    }
    println!("bye bye");
}
