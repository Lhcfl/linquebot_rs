use crate::linquebot::types::*;
use crate::linquebot::*;
use chrono::Utc;
use log::{trace, warn};
use teloxide_core::types::{AllowedUpdate, Update, UpdateKind};

fn update_name(upd: &UpdateKind) -> &'static str {
    match upd {
        UpdateKind::Message(_) => "Message",
        UpdateKind::EditedMessage(_) => "EditedMessage",
        UpdateKind::ChannelPost(_) => "ChannelPost",
        UpdateKind::EditedChannelPost(_) => "EditedChannelPost",
        UpdateKind::MessageReaction(_) => "MessageReaction",
        UpdateKind::MessageReactionCount(_) => "MessageReactionCount",
        UpdateKind::InlineQuery(_) => "InlineQuery",
        UpdateKind::ChosenInlineResult(_) => "ChosenInlineResult",
        UpdateKind::CallbackQuery(_) => "CallbackQuery",
        UpdateKind::ShippingQuery(_) => "ShippingQuery",
        UpdateKind::PreCheckoutQuery(_) => "PreCheckoutQuery",
        UpdateKind::Poll(_) => "Poll",
        UpdateKind::PollAnswer(_) => "PollAnswer",
        UpdateKind::MyChatMember(_) => "MyChatMember",
        UpdateKind::ChatMember(_) => "ChatMember",
        UpdateKind::ChatJoinRequest(_) => "ChatJoinRequest",
        UpdateKind::ChatBoost(_) => "ChatBoost",
        UpdateKind::RemovedChatBoost(_) => "RemovedChatBoost",
        UpdateKind::Error(_) => "Error (ParseError)",
    }
}

pub static ALLOWED_UPDATES: &[AllowedUpdate] = &[
    AllowedUpdate::Message,
    AllowedUpdate::CallbackQuery,
    AllowedUpdate::MyChatMember,
];

pub async fn resolve(app: &'static App, update: Update) {
    let now = Utc::now();

    macro_rules! handle_kind {
        ($kind:ident, $data:ident) => {
            for task in app.micro_tasks {
                if let MicroTask::$kind(task) = task {
                    let task_result = task(app, &$data);
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
        };
    }
    match update.kind {
        UpdateKind::Message(message) => {
            if now.signed_duration_since(message.date).num_seconds() > 30 {
                warn!(
                    target: "main-loop",
                    "skipped message {}s ago: {:?}",
                    now.signed_duration_since(message.date).num_seconds(),
                    message.text()
                );
                return;
            }
            super::message::resolve(app, message);
        }
        UpdateKind::CallbackQuery(data) => {
            trace!("get callback query: {:?}", data.data);
            handle_kind!(OnCallbackQuery, data)
        }
        UpdateKind::MyChatMember(data) => handle_kind!(OnMyChatMember, data),
        _ => {
            warn!(
                "get unimplemented update kind: {:?}",
                update_name(&update.kind)
            );
        }
    }
}
