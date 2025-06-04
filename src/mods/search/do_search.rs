use super::{embedding::text_embedding, toggle::Search};
use crate::{
    linquebot::{
        msg_context::Context, types::Consumption, vector_db::VectorQuery, Module,
        ModuleDescription, ModuleKind,
    },
    utils::telegram::prelude::WarnOnError,
};
use log::{debug, info, warn};
use teloxide_core::{prelude::Request, types::Message};

fn on_search(ctx: &mut Context, msg: &Message) -> Consumption {
    let text = ctx.cmd?.content.to_owned();
    let ctx = ctx.task();
    let vector_db = match &ctx.app.vector_db {
        None => {
            debug!("Vector DB is not initialized, skipping message recording");
            return Consumption::just_next();
        }
        Some(db) => db,
    };
    async move {
        let enabled = ctx
            .app
            .db
            .of::<Search>()
            .chat(ctx.chat_id)
            .get_or_insert(|| Search::default())
            .await
            .search_enabled;
        if !enabled {
            ctx.reply("搜索功能尚未启用")
                .send()
                .warn_on_error("search")
                .await;
            return;
        }
        let embedding = match text_embedding(&text).await {
            Ok(embedding) => embedding,
            Err(e) => {
                warn!("Text Embedding Error with:\n{e}");
                ctx.reply("Text Embedding 发生了内部错误")
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
                ctx.reply("搜索发生了内部错误")
                    .send()
                    .warn_on_error("search")
                    .await;
                return;
            }
            Ok(i) => i,
        };
        info!("{:?}", results);
        ctx.reply(text).send().warn_on_error("search").await;
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
