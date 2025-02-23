use crate::{App, ModuleKind};
use log::{error, trace};
use teloxide_core::types::Message;

pub fn resolve(app: &'static App, message: Message) {
    if let Some(text) = message.text() {
        trace!(target: "main-loop", "get message: {text}");
    }
    if let Some(sticker) = message.sticker() {
        trace!(target: "main-loop", "get sticker: {sticker:?}");
    }
    let mut context = app.create_message_context(&message);
    for module in app.modules {
        if let ModuleKind::Command(desc) = &module.kind {
            if !context.matches_command(desc) {
                continue;
            }
        }
        let task_result = (module.task)(&mut context, &message);
        for task in task_result.tasks {
            tokio::spawn(async move {
                let result = tokio::spawn(task);
                let Err(err) = result.await else {
                    return;
                };
                if err.is_panic() {
                    error!("module {:?} panicked: {err}", module.name());
                }
            });
        }
        if !task_result.next {
            break;
        }
    }
}
