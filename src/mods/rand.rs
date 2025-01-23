use log::warn;
use rand::seq::IteratorRandom;
use rand::Rng;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::utils::*;
use crate::Consumption;

async fn send_raw_rand(bot: &Bot, message: Message, text_body: String) {
    let result = rand::thread_rng().gen_range(0..=100);

    let Some(from) = message.from.clone() else {
        warn!("no reply target");
        return;
    };

    let msg = format!(
        "{} {}",
        from.html_link(),
        if text_body.trim().is_empty() {
            format!("掷出了: {result}")
        } else {
            format!("{} 的概率是: {result}%", escape_html(text_body.trim()))
        }
    );

    if let Err(err) = bot
        .send_message(message.chat.id, msg)
        .parse_mode(ParseMode::Html)
        .send()
        .await
    {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

async fn send_selective_rand(bot: &Bot, message: Message, text_body: String, spliter: &str) {
    let result = text_body
        .split(&spliter)
        .choose(&mut rand::thread_rng())
        .unwrap_or("undefined");

    if let Err(err) = bot
        .send_message(message.chat.id, format!("{}!", escape_html(result)))
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(message.id))
        .send()
        .await
    {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

pub fn on_message(app: &'static App, message: &Message) -> Consumption {
    let text = parse_command(message.text()?, "rand")?.to_string();
    let message = message.clone();

    if text.contains("还是") {
        Consumption::StopWith(Box::pin(send_selective_rand(
            &app.bot, message, text, "还是",
        )))
    } else {
        Consumption::StopWith(Box::pin(send_raw_rand(&app.bot, message, text)))
    }
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "rand",
        description: "抛抛骰子",
        description_detailed: None,
    }),
    task: on_message,
};
