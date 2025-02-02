//! 塔罗牌 AI

use log::trace;
use log::warn;
use msg_context::Context;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::tarot;
use crate::linquebot::*;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

#[derive(Serialize, Deserialize, Debug)]
struct TarotRequestItem {
    no: u8,
    #[serde(rename = "isReversed")]
    is_reversed: bool,
}

/// 它太脏了，但我没办法
fn parse_answer(str: &str) -> String {
    let regex = Regex::new(r"\n(\d+):([^\n]+)").unwrap();

    let mut res_arr = Vec::<String>::new();

    for caps in regex.captures_iter(str) {
        let Some(json_str) = caps.get(2) else {
            continue;
        };
        let json_str = json_str.as_str();
        let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(json_str) else {
            continue;
        };
        let temp_json = match parsed_json.get("diff") {
            Some(diff) if diff.is_array() && diff.get(1).is_some() => diff[1].as_str(),
            _ => parsed_json
                .get("curr")
                .map(|v| v.as_str())
                .unwrap_or_default(),
        };
        let Some(temp_json) = temp_json else {
            continue;
        };

        if !temp_json.is_empty() {
            res_arr.push(temp_json.to_string());
        }
    }

    res_arr.join("")
}

async fn get_tarot(question: &str) -> anyhow::Result<String> {
    let body = format!("接下来的回答请使用中文。我的问题是：{question}");
    let client = reqwest::Client::new();

    let tarots = tarot::n_random_majors(3)
        .into_iter()
        .map(|t| TarotRequestItem {
            no: t.id,
            is_reversed: t.is_reverse,
        })
        .collect::<Vec<_>>();

    let body = format!("[\"{body}\", {}]", serde_json::to_string(&tarots)?);
    trace!("Body: {body}");

    let res = client
        .post("https://yesnotarot.org/")
        .header("Accept", "text/x-component")
        .header("Next-Action", "1d9b84497857784d00b4511601b1ca97cc82c9ac")
        .header("Next-Router-State-Tree", "%5B%22%22%2C%7B%22children%22%3A%5B%5B%22locale%22%2C%22en%22%2C%22d%22%5D%2C%7B%22children%22%3A%5B%22__PAGE__%3F%7B%5C%22locale%5C%22%3A%5C%22en%5C%22%7D%22%2C%7B%7D%5D%7D%5D%7D%2Cnull%2Cnull%2Ctrue%5D")
        .header("Referer", "https://yesnotarot.org/")
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    trace!("Get Tarot Response: {res}");

    let parsed = parse_answer(&res);

    Ok(if parsed.is_empty() { res } else { parsed })
}

fn send_tarot(ctx: &mut Context, _message: &Message) -> Consumption {
    let question = ctx.cmd?.content.to_string();
    let ctx = ctx.task();

    if question.to_string().is_empty() {
        return Consumption::StopWith(Box::pin(
            ctx.reply("必须要有参数哦，参数是 YES OR NO 的一个问题。")
                .send()
                .warn_on_error("tarot"),
        ));
    }

    Consumption::StopWith(Box::pin(async move {
        let placeholder = match ctx.reply("少女祈祷中…").send().await {
            Ok(msg) => msg,
            Err(err) => {
                warn!("Failed to send reply: {}", err.to_string());
                return;
            }
        };

        ctx.app
            .bot
            .send_chat_action(ctx.chat_id, ChatAction::Typing)
            .send()
            .warn_on_error("tarot-ai")
            .await;

        match get_tarot(&question).await {
            Ok(answer) => {
                tokio::spawn(
                    ctx.app
                        .bot
                        .delete_message(ctx.chat_id, placeholder.id)
                        .send()
                        .warn_on_error("tarot-ai"),
                );

                ctx.reply(&answer).send().warn_on_error("tarot-ai").await;
            }
            Err(err) => {
                warn!("get-tarot error: {}", err.to_string());
                ctx.app
                    .bot
                    .edit_message_text(
                        ctx.chat_id,
                        placeholder.id,
                        format!("少女祈祷失败 >.<\n{}", err),
                    )
                    .send()
                    .warn_on_error("tarot-ai")
                    .await;
            }
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "tarot_ai",
        description: "塔罗牌（AI版）",
        description_detailed: Some(concat!("必选参数：提出的问题（最好是 YES OR NO 能回答的）")),
    }),
    task: send_tarot,
};
