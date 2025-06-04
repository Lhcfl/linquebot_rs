use super::toggle::Search;
use crate::{
    linquebot::{msg_context::Context, types::Consumption, vector_db::VectorData, Module},
    mods::search::embedding::text_embedding,
};
use log::{debug, warn};
use teloxide_core::types::Message;

fn on_message(ctx: &mut Context, msg: &Message) -> Consumption {
    let ctx = ctx.task();
    let vector_db = match &ctx.app.vector_db {
        None => {
            debug!("Vector DB is not initialized, skipping message recording");
            return Consumption::just_next();
        }
        Some(db) => db,
    };
    if let Some(text) = msg.text() {
        let text = text.to_owned();
        return Consumption::next_with(async move {
            let enabled = ctx
                .app
                .db
                .of::<Search>()
                .chat(ctx.chat_id)
                .get_or_insert(Search::default)
                .await
                .search_recording_enabled;
            if !enabled {
                return;
            }
            let embedding = match text_embedding(text).await {
                Ok(embedding) => embedding,
                Err(e) => {
                    warn!("Text Embedding Error with:\n{e}");
                    return;
                }
            };
            let res = vector_db
                .upsert(VectorData {
                    chat: ctx.chat_id.to_string(),
                    index: ctx.message_id.to_string(),
                    user: None,
                    vector: embedding,
                })
                .await;
            if res.is_err() {
                warn!("Failed to upsert vector data");
            }
        });
    }
    Consumption::just_next()
}

pub static RECORDER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
