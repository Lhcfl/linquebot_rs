//! explain
//! 调用各种API解释一个词

use log::warn;
use msg_context::Context;
use serde::Deserialize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::idiom::get_idiom;
use crate::linquebot::*;
use crate::utils::sanitize_html;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

#[derive(Deserialize, Debug)]
struct WikiResponse {
    extract_html: String,
}

async fn get_from_wikipedia(title: &str) -> Result<(String, String), reqwest::Error> {
    let rep = title.replace("_", " ");
    let encoded = urlencoding::encode(&rep);
    let res = reqwest::get(format!(
        "https://zh.wikipedia.org/api/rest_v1/page/summary/{encoded}",
    ))
    .await?
    .json::<WikiResponse>()
    .await?;

    Ok((
        format!("<a href=\"https://zh.wikipedia.org/wiki/{encoded}\">维基百科</a>: "),
        sanitize_html(&res.extract_html),
    ))
}

fn send_explain(ctx: &mut Context, _: &Message) -> Consumption {
    let args = ctx.cmd?.content;
    if args.is_empty() {
        return ctx
            .task()
            .reply("请输入至少一个参数")
            .send()
            .warn_on_error("explain")
            .into();
    }

    let query = args.to_string();
    let ctx = ctx.task();
    async move {
        let app = ctx.app;
        let chat_id = ctx.chat_id;
        tokio::spawn(async move {
            app.bot
                .send_chat_action(chat_id, ChatAction::Typing)
                .send()
                .warn_on_error("explain-send-action")
                .await;
        });

        let mut answers = Vec::<String>::new();

        // explain for idioms
        if let Some(idiom) = get_idiom(&query) {
            answers.push(idiom.html_description());
        }

        match get_from_wikipedia(&query).await {
            Ok((src, explain)) => answers.push(src + &explain),
            Err(err) => warn!("failed to fetch wikipedia: {}", err.to_string()),
        }

        let ans = answers.join("\n\n");

        if ans.is_empty() {
            ctx.reply("没有找到解释呢")
                .send()
                .warn_on_error("explain-reply")
                .await;
        } else {
            ctx.reply_html(&ans)
                .send()
                .warn_on_error("explain-reply")
                .await;
        }
    }
    .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "explain",
        description: "解释名词",
        description_detailed: Some("需要一个参数，即等待解释的名词"),
    }),
    task: send_explain,
};
