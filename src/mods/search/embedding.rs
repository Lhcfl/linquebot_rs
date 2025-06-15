use super::qwen3_embedding::{Config, Model};
use anyhow::{Error as E, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use hf_hub::{
    api::sync::{Api, ApiBuilder},
    Repo, RepoType,
};
use std::sync::LazyLock;
use tokenizers::{Tokenizer, TruncationDirection, TruncationParams, TruncationStrategy};
use tokio::sync::Mutex;

static MODEL_ID: &str = "Qwen/Qwen3-Embedding-0.6B";
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
        .with_truncation(Some(TruncationParams {
            max_length: 8192,
            strategy: TruncationStrategy::default(),
            stride: 0,
            direction: TruncationDirection::default(),
        }))
        .map_err(E::msg)?
        .to_owned()
        .into();
    Ok(tokenizer)
}

fn get_model() -> Result<Mutex<Model>> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo);
    let model_filename = api.get("model.safetensors")?;
    let config_filename = api.get("config.json")?;
    let config: Config = serde_json::from_slice(&std::fs::read(config_filename)?)?;
    let device = candle_core::Device::Cpu;
    let dtype = DType::F32;

    let model_safetensors = std::fs::read(model_filename)?;
    // let filenames = vec![model_filename];
    let vb = VarBuilder::from_slice_safetensors(&model_safetensors, dtype, &device)?;

    let model = Model::new(&config, vb)?;
    Ok(Mutex::new(model))
}

static TOKENIZER: LazyLock<Result<Tokenizer>> = LazyLock::new(get_tokenizer);
static MODEL: LazyLock<Result<Mutex<Model>>> = LazyLock::new(get_model);

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let tokenizer = TOKENIZER.as_ref().map_err(E::msg)?;
    let model_mutex = MODEL.as_ref().map_err(E::msg)?;
    let mut model = model_mutex.lock().await;
    let encoding = tokenizer.encode(text.into(), true).map_err(E::msg)?;
    let inputs = encoding.get_ids();
    let tokens = Tensor::new(inputs, &Device::Cpu)?.reshape((inputs.len(), ()))?;
    let outputs = model.forward(&tokens, inputs.len())?;
    Ok(outputs.get(inputs.len() - 1)?.get(0)?.to_vec1::<f32>()?)
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
        let text2 = "Hello, universe!";
        let embedding = text_embedding(text).await?;
        assert_eq!(embedding.len(), 1024); // Assuming the model outputs 1024-dimensional embeddings
        let embedding_2 = text_embedding(text2).await?;
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
