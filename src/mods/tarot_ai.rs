//! 塔罗牌 AI

use log::trace;
use log::warn;
use msg_context::Context;
use regex::Regex;
use serde::Serialize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::tarot;
use crate::linquebot::config::ConfigAiApi;
use crate::linquebot::*;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

#[derive(Serialize)]
enum AiRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
}

#[derive(Serialize)]
struct AiMessage {
    role: AiRole,
    content: String,
}

#[derive(Serialize)]
struct AiRequestBody<'a> {
    model: &'a String,
    messages: [AiMessage; 2],
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

async fn get_tarot(question: &str, config_ai_api: &ConfigAiApi) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let tarots = tarot::n_random_majors(3)
        .into_iter()
        .map(|t| {
            format!(
                "序号：{}，是否反转：{}",
                t.id,
                if t.is_reverse { "是" } else { "否" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let body = format!("我的问题：\n```\n{question}\n```");
    let body = format!("{body}\n我抽取到的塔罗牌：\n```\n{tarots}\n```");
    let body: [AiMessage; 2] = [
        AiMessage {
            role: AiRole::System,
            content: "请在接下来根据我的问题和我抽取到的塔罗牌进行回答。".to_string(),
        },
        AiMessage {
            role: AiRole::User,
            content: body,
        },
    ];
    let body = AiRequestBody {
        model: &config_ai_api.model,
        messages: body,
    };

    let body = serde_json::to_string(&body)?;
    trace!("Body: {body}");

    let res = client
        .post(&config_ai_api.url)
        .header("Authorization", format!("Bearer {}", &config_ai_api.token))
        .header("Content-Type", "application/json")
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
        return ctx
            .reply("必须要有参数哦，参数是 YES OR NO 的一个问题。")
            .send()
            .warn_on_error("tarot")
            .into();
    }

    async move {
        let placeholder = match ctx.reply("少女祈祷中…").send().await {
            Ok(msg) => msg,
            Err(err) => {
                warn!("Failed to send reply: {}", err);
                return;
            }
        };

        ctx.app
            .bot
            .send_chat_action(ctx.chat_id, ChatAction::Typing)
            .send()
            .warn_on_error("tarot-ai")
            .await;

        match get_tarot(&question, &ctx.app.config.ai.api).await {
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
                warn!("get-tarot error: {}", err);
                ctx.app
                    .bot
                    .edit_message_text(
                        ctx.chat_id,
                        placeholder.id,
                        format!("少女祈祷失败 >.<\n{err}"),
                    )
                    .send()
                    .warn_on_error("tarot-ai")
                    .await;
            }
        }
    }
    .into()
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "tarot_ai",
        description: "塔罗牌（AI版）",
        description_detailed: Some("必选参数：提出的问题（最好是 YES OR NO 能回答的）"),
    }),
    task: send_tarot,
};
