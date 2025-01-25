use log::warn;
use teloxide_core::prelude::*;
use teloxide_core::types::{
    CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode,
};

use crate::msg_context::Context;
use crate::{App, Consumption, MicroTask, Module, ModuleDesctiption, ModuleKind};

fn read_description(kind: &ModuleKind) -> Option<&ModuleDesctiption> {
    match kind {
        ModuleKind::Command(cmd) => Some(cmd),
        ModuleKind::General(Some(cmd)) => Some(cmd),
        _ => None,
    }
}

static HELP_HEAD: &str = "OoO 这里是琳酱的帮助";

fn gen_help_message(app: &App) -> (String, InlineKeyboardMarkup) {
    let mut command_texts = Vec::<String>::new();
    let mut general_texts = Vec::<String>::new();
    let mut detailed_modules = Vec::<&str>::new();

    for module in app.modules {
        match &module.kind {
            ModuleKind::Command(cmd) => {
                command_texts.push(format!("/{}: {}", cmd.name, cmd.description));
                if let Some(_) = cmd.description_detailed {
                    detailed_modules.push(cmd.name);
                }
            }
            ModuleKind::General(Some(cmd)) => {
                general_texts.push(format!("**{}**: {}", cmd.name, cmd.description,));
                if let Some(_) = cmd.description_detailed {
                    detailed_modules.push(cmd.name);
                }
            }
            _ => {}
        }
    }

    let mut keyboards_iter = detailed_modules
        .into_iter()
        .map(|module_name| {
            InlineKeyboardButton::callback(module_name, format!("help {module_name}"))
        })
        .array_chunks::<3>();

    let mut keyboards: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    loop {
        if let Some(x) = keyboards_iter.next() {
            keyboards.push(x.into_iter().collect::<Vec<_>>());
        } else {
            if let Some(x) = keyboards_iter.into_remainder() {
                keyboards.push(x.collect::<Vec<_>>());
            }
            break;
        }
    }

    let message = format!(
        "{HELP_HEAD}:\n{}\n\n{}\n\n点击下方按钮可以看到对应模块的详细帮助",
        command_texts.join("\n"),
        general_texts.join("\n")
    );

    (message, InlineKeyboardMarkup::new(keyboards))
}

fn send_help(ctx: &mut Context, _msg: &Message) -> Consumption {
    let ctx = ctx.task();
    Consumption::StopWith(Box::pin(async move {
        let (message, reply_markup) = gen_help_message(ctx.app);

        let res = ctx
            .reply_markdown(&message)
            .reply_markup(reply_markup)
            .send()
            .await;

        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

fn on_help_callback(app: &'static App, cq: &CallbackQuery) -> Consumption {
    use crate::utils::pattern::*;
    let (_, (_, help_module_name)) =
        ("help ", of_pred(|_| true)).check_pattern(cq.data.as_ref()?)?;

    let message = cq.message.clone()?;
    let chat_id = message.chat().id;

    for module in app.modules {
        let Some(desc) = read_description(&module.kind) else {
            continue;
        };
        if desc.name == help_module_name && desc.description_detailed.is_some() {
            return Consumption::StopWith(Box::pin(async move {
                let res = app
                    .bot
                    .edit_message_text(
                        chat_id,
                        message.id(),
                        format!(
                            "{HELP_HEAD}\n\n**{}**: {}\n\n{}",
                            desc.name,
                            desc.description,
                            desc.description_detailed.expect("上面检查了 is_some")
                        ),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(InlineKeyboardMarkup::new(vec![vec![
                        InlineKeyboardButton::callback("返回", "help {default}"),
                    ]]))
                    .send()
                    .await;

                if let Err(err) = res {
                    warn!("Failed to send edit: {}", err.to_string());
                }
            }));
        }
    }

    Consumption::StopWith(Box::pin(async move {
        let (text, reply_markup) = gen_help_message(app);

        let res = app
            .bot
            .edit_message_text(chat_id, message.id(), text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(reply_markup)
            .send()
            .await;

        if let Err(err) = res {
            warn!("Failed to send edit: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "help",
        description: "显示帮助",
        description_detailed: None,
    }),
    task: send_help,
};

pub static HELP_CALLBACK: MicroTask = MicroTask::OnCallbackQuery(on_help_callback);
