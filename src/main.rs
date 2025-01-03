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

static MODULE_HANDLES: &[fn(&Bot, &Message) -> Option<ComsumedType>] = &[mods::rong::on_message];

fn module_resolver(bot: &Bot, message: &Message) -> () {
    println!("text: {:?}", message.text());

    for handle in MODULE_HANDLES {
        if let Some(consumed) = handle(bot, message) {
            if let ComsumedType::Stop = consumed {
                break;
            }
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
    let bot = Bot::from_env();
    let me = bot.get_me().await?;
    println!("user id: {}", me.id);
    println!("user name: {}", me.username.as_ref().unwrap());
    let mut offset: Option<i32> = None;

    loop {
        let mut update = bot.get_updates();
        update.timeout = Some(30000);
        update.offset = offset.clone();
        println!("getting updates");

        match update.send().await {
            Ok(updates) => {
                offset = updates.last().and_then(|u| Some(u.id.0 as i32 + 1));
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
