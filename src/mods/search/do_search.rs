use super::{embedding::text_embedding, toggle::Search};
use crate::{
    linquebot::{
        msg_context::Context,
        types::Consumption,
        vector_db::{VectorQuery, VectorResult},
        Module, ModuleDescription, ModuleKind,
    },
    utils::telegram::prelude::WarnOnError,
};
use log::{debug, warn};
use teloxide_core::{
    prelude::Request,
    types::{ChatId, Message, MessageId},
};

fn vector_result_to_string(r: &VectorResult) -> Option<String> {
    let message_id = MessageId(r.index.parse().ok()?);
    let chat_id = ChatId(r.chat.parse().ok()?);
    let user_id = r.user.as_deref();
    let distance = r.distance;
    match Message::url_of(chat_id, user_id, message_id) {
        None => {
            warn!("Failed to create URL for message: {:?}", r);
            None
        }
        Some(url) => Some(format!("{} {:.4}", url.as_str(), distance)),
    }
}

fn on_search(ctx: &mut Context, _: &Message) -> Consumption {
    let text = ctx.cmd?.content.to_owned();
    let ctx = ctx.task();
    async move {
        let enabled = ctx
            .app
            .db
            .of::<Search>()
            .chat(ctx.chat_id)
            .get_or_insert(Search::default)
            .await
            .search_enabled;
        if !enabled {
            ctx.reply("搜索功能尚未启用")
                .send()
                .warn_on_error("search")
                .await;
            return;
        }
        let vector_db = match &ctx.app.vector_db {
            None => {
                debug!("Vector DB is not initialized, skipping message searching");
                ctx.reply("未连接到向量数据库，无法进行搜索")
                    .send()
                    .warn_on_error("search")
                    .await;
                return;
            }
            Some(db) => db,
        };
        if text.is_empty() {
            ctx.reply("搜索内容不能为空")
                .send()
                .warn_on_error("search")
                .await;
            return;
        }
        let embedding = match text_embedding(&text).await {
            Ok(embedding) => embedding,
            Err(e) => {
                warn!("Text Embedding Error with:\n{e}");
                ctx.reply_markdown(format!("词嵌入发生了内部错误\n```\n{e}\n```"))
                    .send()
                    .warn_on_error("search")
                    .await;
                return;
            }
        };
        let results = match vector_db
            .get(VectorQuery {
                chat: ctx.chat_id.to_string(),
                user: None,
                vector: embedding,
            })
            .await
        {
            Err(e) => {
                warn!("Query Failed with:\n{e}");
                ctx.reply_markdown(format!("搜索发生了内部错误\n```\n{e}\n```"))
                    .send()
                    .warn_on_error("search")
                    .await;
                return;
            }
            Ok(i) => i,
        };
        let links = results
            .iter()
            .filter_map(vector_result_to_string)
            .collect::<Vec<String>>()
            .join("\n");
        ctx.reply(links).send().warn_on_error("search").await;
    }
    .into()
}

pub static SEARCH: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "search",
        description: "搜索内容",
        description_detailed: None,
    }),
    task: on_search,
};
