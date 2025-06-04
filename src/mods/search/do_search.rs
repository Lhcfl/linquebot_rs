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
use ammonia::Url;
use log::{debug, warn};
use teloxide_core::{
    prelude::Request,
    types::{ChatId, Message, MessageId},
};

fn vector_result_to_link(r: &VectorResult) -> anyhow::Result<Url> {
    let message_id = MessageId(r.index.parse()?);
    let chat_id = ChatId(r.chat.parse()?);
    let user_id = r.user.as_ref().map(|s| s.as_str());
    match Message::url_of(chat_id, user_id, message_id) {
        None => {
            warn!("Failed to create URL for message: {:?}", r);
            Err(anyhow::anyhow!("Failed to create URL for message"))
        }
        Some(url) => Ok(url),
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
            .map(vector_result_to_link)
            .filter_map(|item| match item {
                Ok(url) => Some(url.to_string()),
                Err(e) => {
                    warn!("Failed to convert vector result to link: {}", e);
                    None
                }
            })
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
