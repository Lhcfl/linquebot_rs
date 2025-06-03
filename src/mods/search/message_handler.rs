use super::toggle::Search;
use crate::linquebot::{msg_context::Context, types::Consumption, vector_db::VectorData, Module};
use log::{debug, warn};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use teloxide_core::types::Message;

#[derive(Serialize)]
struct EmbeddingReq<'a> {
    model: &'a str,
    prompt: String,
}

#[derive(Deserialize)]
struct EmbeddingRes {
    embedding: Vec<f64>,
}

static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .build()
        .expect("Cannot build reqwest client for search")
});

static OLLAMA_EMBEDDING_API_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("OLLAMA_EMBEDDING_API_URL")
        .unwrap_or("http://localhost:11434/api/embeddings".to_string())
});

static OLLAMA_EMBEDDING_MODEL: LazyLock<String> =
    LazyLock::new(|| std::env::var("OLLAMA_EMBEDDING_MODEL").unwrap_or("bge-m3".to_string()));

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
        let prompt = text.to_owned();
        return Consumption::next_with(async move {
            let enabled = ctx
                .app
                .db
                .of::<Search>()
                .chat(ctx.chat_id)
                .get_or_insert(|| Search::default())
                .await
                .search_recording_enabled;
            if !enabled {
                return;
            }
            let res = REQWEST_CLIENT
                .post(OLLAMA_EMBEDDING_API_URL.as_str())
                .json(&EmbeddingReq {
                    model: &OLLAMA_EMBEDDING_MODEL.as_str(),
                    prompt,
                })
                .send()
                .await;
            let res = match res {
                Err(e) => Err(e),
                Ok(res) => res.json::<EmbeddingRes>().await,
            };
            let embedding = match res {
                Err(e) => {
                    warn!("Failed to get embedding for message, error:\n{}", e);
                    return;
                }
                Ok(res) => res.embedding,
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

pub static MESSAGE_HANDLER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
