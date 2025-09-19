//! 塔罗牌 AI

use log::trace;
use log::warn;
use msg_context::Context;
use serde::Deserialize;
use serde::Serialize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::assets::tarot;
use crate::linquebot::config::Config;
use crate::linquebot::*;
use crate::utils::telegram::prelude::WarnOnError;
use crate::Consumption;

#[derive(Debug, Serialize, Deserialize)]
enum AiRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Serialize, Deserialize)]
struct AiMessage {
    role: AiRole,
    content: String,
}

#[derive(Serialize)]
struct AiRequestBody {
    model: String,
    messages: [AiMessage; 2],
}

#[derive(Debug, Deserialize)]
struct AiResponseBody {
    choices: [AiResponseChoice; 1],
}

#[derive(Debug, Deserialize)]
struct AiResponseChoice {
    message: AiMessage,
}

async fn get_tarot(question: &str, config: &Config) -> anyhow::Result<String> {
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
            content: config.tarot.ai.prompt.clone(),
        },
        AiMessage {
            role: AiRole::User,
            content: body,
        },
    ];
    let body = AiRequestBody {
        model: config.ai.api.model.clone(),
        messages: body,
    };

    let body = serde_json::to_string(&body)?;
    trace!("Body: {body}");

    let res = client
        .post(&config.ai.api.url)
        .header("Authorization", format!("Bearer {}", &config.ai.api.token))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?
        .text()
        .await?;
    let res_json = serde_json::from_str::<AiResponseBody>(&res);

    trace!("Get Tarot Response: {:#?}", res);

    match res_json {
        Err(err) => {
            warn!("Couldn't parse tarot ai response:\n{err}");
            Ok(res)
        }
        Ok(json) => Ok(json.choices[0].message.content.clone()),
    }
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

        match get_tarot(&question, &ctx.app.config).await {
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
