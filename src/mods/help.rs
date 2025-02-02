/// 显示帮助和关于信息
use teloxide_core::prelude::*;
use teloxide_core::types::{
    CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, LinkPreviewOptions, Message,
    ParseMode,
};

use crate::msg_context::Context;
use crate::utils::telegram::prelude::WarnOnError;
use crate::{App, Consumption, MicroTask, Module, ModuleDescription, ModuleKind};

fn read_description(kind: &ModuleKind) -> Option<&ModuleDescription> {
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
                if cmd.description_detailed.is_some() {
                    detailed_modules.push(cmd.name);
                }
            }
            ModuleKind::General(Some(cmd)) => {
                general_texts.push(format!("<b>{}</b>: {}", cmd.name, cmd.description,));
                if cmd.description_detailed.is_some() {
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
        "{HELP_HEAD}
<blockquote expandable>命令列表:
{}

模块列表:
{}
</blockquote>
⬆️ 点击展开命令菜单

琳酱以 AGPL 开源于 https://github.com/Lhcfl/linquebot_rs/
欢迎检查源代码和点星星✨

点击下方按钮可以看到对应模块的详细帮助",
        command_texts.join("\n"),
        general_texts.join("\n")
    );

    (message, InlineKeyboardMarkup::new(keyboards))
}

fn gen_partial_help_message(
    app: &App,
    module_name: &str,
) -> Option<(String, InlineKeyboardMarkup)> {
    for module in app.modules {
        let Some(desc) = read_description(&module.kind) else {
            continue;
        };
        if desc.name == module_name && desc.description_detailed.is_some() {
            return Some((
                format!(
                    "{HELP_HEAD}\n\n<b>{}</b>: {}\n\n{}",
                    desc.name,
                    desc.description,
                    desc.description_detailed.expect("上面检查了 is_some")
                ),
                InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
                    "返回",
                    "help {default}",
                )]]),
            ));
        }
    }
    None
}

fn disabled_link_preview() -> LinkPreviewOptions {
    LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_large_media: false,
        prefer_small_media: false,
        show_above_text: false,
    }
}

fn send_help(ctx: &mut Context, _msg: &Message) -> Consumption {
    let module_name = ctx.cmd?.content;
    let ctx = ctx.task();
    let (msg, btn) =
        gen_partial_help_message(ctx.app, module_name).unwrap_or_else(|| gen_help_message(ctx.app));
    ctx.reply_html(msg)
        .reply_markup(btn)
        .link_preview_options(disabled_link_preview())
        .send()
        .warn_on_error("help")
        .into()
}

fn say_hi(ctx: &mut Context, msg: &Message) -> Consumption {
    let news = msg.new_chat_members()?;
    if news.iter().any(|member| member.id == ctx.app.bot_id) {
        ctx.app
            .bot
            .send_message(
                ctx.chat_id,
                concat!(
                    "大家好！这里是琳酱 ♪(´▽｀)\n",
                    "琳酱是多功能的开源群聊机器人，提供小游戏、简单命令、实用工具等\n",
                    "琳酱需要消息权限和设置管理员权限来解锁全部模块功能\n\n",
                    "使用 /help 查看详细琳酱帮助"
                ),
            )
            .link_preview_options(disabled_link_preview())
            .send()
            .warn_on_error("say-hi")
            .into()
    } else {
        Consumption::Next
    }
}

fn on_help_callback(app: &'static App, cq: &CallbackQuery) -> Consumption {
    use crate::utils::pattern::*;
    let (_, (_, help_module_name)) =
        ("help ", of_pred(|_| true)).check_pattern(cq.data.as_ref()?)?;

    let message = cq.message.clone()?;
    let chat_id = message.chat().id;

    let (msg, btn) =
        gen_partial_help_message(app, help_module_name).unwrap_or_else(|| gen_help_message(app));

    app.bot
        .edit_message_text(chat_id, message.id(), msg)
        .parse_mode(ParseMode::Html)
        .link_preview_options(disabled_link_preview())
        .reply_markup(btn)
        .send()
        .warn_on_error("edit-help")
        .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "help",
        description: "显示帮助",
        description_detailed: None,
    }),
    task: send_help,
};

pub static SAY_HI: Module = Module {
    kind: ModuleKind::General(None),
    task: say_hi,
};

pub static HELP_CALLBACK: MicroTask = MicroTask::OnCallbackQuery(on_help_callback);
