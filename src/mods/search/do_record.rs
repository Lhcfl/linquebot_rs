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
        Err(_) => {
            debug!("Vector DB is not initialized, skipping message recording");
            return Consumption::just_next();
        }
        Ok(db) => db,
    };
    let Some(text) = msg.text() else {
        return Consumption::just_next();
    };

    let text = text.to_owned();
    Consumption::next_with(async move {
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
        let embedding = match text_embedding(text) {
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
    })
}

pub static RECORDER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
