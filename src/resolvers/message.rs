use crate::{linquebot::types::*, App, ModuleKind};
use log::{error, trace};
use teloxide_core::types::Message;

pub fn resolve(app: &'static App, message: Message) {
    trace!(target: "main-loop", "get message: {:?}", message.text());
    let mut context = app.create_message_context(&message);
    for module in app.modules {
        if let ModuleKind::Command(desc) = &module.kind {
            if !context.matches_command(desc) {
                continue;
            }
        }
        let task_result = (module.task)(&mut context, &message);
        match task_result {
            Consumption::Next => {}
            Consumption::Stop => {
                break;
            }
            Consumption::StopWith(task) => {
                tokio::spawn(async move {
                    let result = tokio::spawn(task);
                    let Err(err) = result.await else {
                        return;
                    };
                    if err.is_panic() {
                        error!("module {:?} panicked: {err}", module.name());
                    }
                });
                break;
            }
        }
    }
}
