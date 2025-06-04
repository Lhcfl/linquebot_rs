use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;
/// 随机选择器
use msg_context::Context;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

fn on_debugger(ctx: &mut Context, message: &Message) -> Consumption {
    ctx.task()
        .reply_html(format!(
            "群组 ID: <code>{}</code>
消息 ID: <code>{}</code>
详细信息：
<pre><code class=\"language-rust\">
{:?}
</code></pre>",
            ctx.chat_id, message.id, message
        ))
        .send()
        .warn_on_error("debugger-mid")
        .into()
}

pub static DEBUGGER: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "debugger",
        description: "调试器",
        description_detailed: None,
    }),
    task: on_debugger,
};
