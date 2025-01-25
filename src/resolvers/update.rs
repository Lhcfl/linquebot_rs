use std::any::Any;

use crate::linquebot::types::*;
use crate::linquebot::*;
use chrono::Utc;
use log::{trace, warn};
use teloxide_core::types::{Update, UpdateKind};

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
            if now.signed_duration_since(&message.date).num_seconds() > 30 {
                warn!(
                    target: "main-loop",
                    "skipped message {}s ago: {:?}",
                    now.signed_duration_since(&message.date).num_seconds(),
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
        UpdateKind::ChatMember(data) => handle_kind!(OnChatMember, data),
        UpdateKind::MyChatMember(data) => handle_kind!(OnChatMember, data),
        _ => {
            warn!("get unimplemented update kind: {:?}", update.kind.type_id());
        }
    }
}
