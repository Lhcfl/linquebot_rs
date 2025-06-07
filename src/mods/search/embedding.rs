use anyhow::{Error as E, Result};
use hf_hub::{
    api::sync::{Api, ApiBuilder},
    Repo, RepoType,
};
use ndarray::Ix1;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use std::sync::LazyLock;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

static MODEL_ID: &str = "Snowflake/snowflake-arctic-embed-l-v2.0";
static REVISION: &str = "main";

fn get_tokenizer() -> Result<Tokenizer> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = ApiBuilder::new()
        .with_cache_dir("cache/huggingface".into())
        .build()?;
    let api = api.repo(repo);
    let tokenizer_filename = api.get("tokenizer.json")?;
    let tokenizer = Tokenizer::from_file(tokenizer_filename)
        .map_err(E::msg)?
        .with_padding(None)
        .with_truncation(None)
        .map_err(E::msg)?
        .to_owned()
        .into();
    Ok(tokenizer)
}

fn get_session() -> Result<Mutex<Session>> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo);
    let onnx_filename = api.get("onnx/model_uint8.onnx")?;
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(onnx_filename)?;
    Ok(Mutex::new(session))
}

static TOKENIZER: LazyLock<Result<Tokenizer>> = LazyLock::new(get_tokenizer);
static SESSION: LazyLock<Result<Mutex<Session>>> = LazyLock::new(get_session);

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let tokenizer = TOKENIZER.as_ref().map_err(E::msg)?;
    let session_mutex = SESSION.as_ref().map_err(E::msg)?;
    let mut session = session_mutex.lock().await;
    let encoding = tokenizer.encode(text.into(), true).map_err(E::msg)?;
    let tokens = encoding
        .get_ids()
        .iter()
        .map(|&t| t.into())
        .collect::<Vec<i64>>();
    let attention_mask = encoding
        .get_attention_mask()
        .iter()
        .map(|&m| m.into())
        .collect::<Vec<i64>>();
    let tokens = TensorRef::from_array_view(([1, tokens.len()], tokens.as_slice()))?;
    let attention_mask =
        TensorRef::from_array_view(([1, attention_mask.len()], attention_mask.as_slice()))?;
    let outputs = session.run(ort::inputs![tokens, attention_mask])?;
    let embeddings = outputs[1]
        .try_extract_array()?
        .squeeze()
        .into_dimensionality::<Ix1>()?
        .to_vec();
    Ok(embeddings)
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;

    #[tokio::test()]
    async fn test_text_embedding() -> Result<()> {
        if std::env::var("CI").is_ok() {
            return Ok(());
        };
        let text = "Hello, world!";
        let embedding = text_embedding(text).await?;
        assert_eq!(embedding.len(), 1024); // Assuming the model outputs 768-dimensional embeddings
        let embedding_2 = text_embedding(text).await?;
        assert_eq!(embedding, embedding_2);
        Ok(())
    }

    #[bench]
    fn bench_text_embedding(b: &mut test::Bencher) {
        if std::env::var("CI").is_ok() {
            return;
        };
        let text = "Hello, world!";
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // rt.block_on(text_embedding(text)).unwrap();
        b.iter(|| {
            rt.block_on(text_embedding(text)).expect("Embedding failed");
        });
    }
}
