use msg_context::Context;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::base64;
use crate::utils::split_args;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

const TOOLS_HELP: &str = concat!(
    "必选参数：工具名\n",
    "<code>/tools base64 [文字]</code> 返回文字的base64\n",
    "<code>/tools base64d [文字]</code> 返回文字的base64解码\n",
);

fn on_message(ctx: &mut Context, _message: &Message) -> Consumption {
    let [toolname, content] = split_args::<2>(ctx.cmd?.content);
    let task = ctx.task();
    match toolname {
        "base64" => {
            let res = format!("`{}`", base64::encode(content));
            if res.len() == 2 {
                task.reply("<空串>")
            } else if res.len() >= 4095 {
                task.reply("你给出的输入太长了！")
            } else {
                task.reply_markdown(&res)
            }
        }
        "base64d" => task.reply(
            base64::decode(content)
                .map(|str| {
                    if str.trim().is_empty() {
                        "<空串>".to_string()
                    } else {
                        str
                    }
                })
                .unwrap_or_else(|err| err.to_string()),
        ),
        "" => task.reply_html(TOOLS_HELP),
        _ => task.reply("未知的工具名"),
    }
    .send()
    .warn_on_error("tools")
    .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "tools",
        description: "实用工具",
        description_detailed: Some(TOOLS_HELP),
    }),
    task: on_message,
};
