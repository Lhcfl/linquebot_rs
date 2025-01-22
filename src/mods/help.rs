use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::{ChatId, Message, MessageId, ParseMode, ReplyParameters};

use crate::{App, Consumption, Module, ModuleDesctiption, ModuleKind};

async fn send_help(app: &App, chat_id: ChatId, message_id: MessageId) {
    let mut command_texts = Vec::<String>::new();
    let mut general_texts = Vec::<String>::new();
    for module in app.modules {
        match &module.kind {
            ModuleKind::Command(cmd) => {
                command_texts.push(format!("/{}: {}", cmd.name, cmd.description));
                if let Some(long_desc) = cmd.description_detailed {
                    command_texts.push(long_desc.to_string());
                }
            }
            ModuleKind::General(Some(cmd)) => {
                general_texts.push(format!("**{}**: {}", cmd.name, cmd.description,));
                if let Some(long_desc) = cmd.description_detailed {
                    general_texts.push(long_desc.to_string());
                }
            }
            _ => {}
        }
    }

    let res = app
        .bot
        .send_message(
            chat_id,
            format!(
                "OoO 这里是琳酱帮助:\n{}\n{}",
                command_texts.join("\n"),
                general_texts.join("\n")
            ),
        )
        .reply_parameters(ReplyParameters::new(message_id))
        .parse_mode(ParseMode::MarkdownV2)
        .await;

    if let Err(err) = res {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

fn on_message(app: &'static App, msg: &Message) -> Consumption {
    let _ = app.parse_command(msg.text()?, "help")?.to_string();
    let chat_id = msg.chat.id;
    let message_id = msg.id;

    Consumption::StopWith(Box::pin(send_help(app, chat_id, message_id)))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "help",
        description: "显示帮助",
        description_detailed: None,
    }),
    task: on_message,
};
