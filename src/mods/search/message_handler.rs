use std::sync::LazyLock;

use crate::linquebot::{msg_context::Context, types::Consumption, Module};
use log::{info, warn};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use teloxide_core::{
    prelude::{Request, Requester},
    types::Message,
};

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
    if let Some(text) = msg.text() {
        let prompt = text.to_owned();
        return Consumption::next_with(async move {
            let result = REQWEST_CLIENT
                .post(OLLAMA_EMBEDDING_API_URL.as_str())
                .json(&EmbeddingReq {
                    model: &OLLAMA_EMBEDDING_MODEL.as_str(),
                    prompt,
                })
                .send()
                .await;
            let res = match result {
                Err(e) => Result::Err(e),
                Ok(res) => res.json::<EmbeddingRes>().await,
            };
            match res {
                Err(e) => {
                    warn!("Text embedding error with {e}")
                }
                Ok(res) => {
                    info!("{:?}", res.embedding)
                }
            }
        });
    }
    Consumption::just_next()
}

pub static MESSAGE_HANDLER: Module = Module {
    kind: crate::linquebot::ModuleKind::General(None),
    task: on_message,
};
