use log::warn;
use rand::seq::SliceRandom;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::answer_book;
use crate::utils::*;
use crate::ComsumedType;

pub fn on_message(bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let _ = parse_command(message.text()?, "answer")?.to_string();
    let bot = bot.clone();
    let message = message.clone();

    tokio::spawn(async move {
        let chosen = answer_book::ANSWERS
            .choose(&mut rand::thread_rng())
            .expect("not empty");
        let res = bot
            .send_message(message.chat.id, *chosen)
            .reply_parameters(ReplyParameters::new(message.id))
            .send()
            .await;
        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    });

    Some(ComsumedType::Stop)
}
