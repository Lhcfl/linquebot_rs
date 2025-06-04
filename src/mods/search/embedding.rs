use std::sync::LazyLock;

use anyhow::Result;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};

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

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f64>> {
    let res = REQWEST_CLIENT
        .post(OLLAMA_EMBEDDING_API_URL.as_str())
        .json(&EmbeddingReq {
            model: OLLAMA_EMBEDDING_MODEL.as_str(),
            prompt: text.into(),
        })
        .send()
        .await?
        .json::<EmbeddingRes>()
        .await?;
    Ok(res.embedding)
}
