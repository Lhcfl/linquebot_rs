use anyhow::{Error as E, Result};
use hf_hub::{api::sync::Api, Repo, RepoType};
use llama_cpp_2::{
    context::params::{LlamaContextParams, LlamaPoolingType},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel},
    send_logs_to_tracing, LogOptions,
};
use std::sync::LazyLock;

static MODEL_ID: &str = "Qwen/Qwen3-Embedding-0.6B-GGUF";
static REVISION: &str = "main";

fn get_model() -> Result<LlamaModel> {
    send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));
    let backend = BACKEND.as_ref().map_err(E::msg)?;
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo);
    let model_filename = api.get("Qwen3-Embedding-0.6B-Q8_0.gguf")?;
    let model = LlamaModel::load_from_file(backend, model_filename, &Default::default())?;
    Ok(model)
}

fn get_backend() -> Result<LlamaBackend> {
    LlamaBackend::init().map_err(E::msg)
}

// fn get_ctx() -> Result<Mutex<LlamaContext<'static>>> {
//     let model = MODEL.as_ref().map_err(E::msg)?;
//     let backend = BACKEND.as_ref().map_err(E::msg)?;
//     let ctx = model.new_context(
//         backend,
//         LlamaContextParams::default()
//             .with_flash_attention(true)
//             .with_pooling_type(LlamaPoolingType::Last)
//             .with_embeddings(true),
//     )?;
//     Ok(Mutex::new(ctx))
// }

static MODEL: LazyLock<Result<LlamaModel>> = LazyLock::new(get_model);
static BACKEND: LazyLock<Result<LlamaBackend>> = LazyLock::new(get_backend);
// static CONTEXT: LazyLock<Result<Mutex<LlamaContext<'static>>>> = LazyLock::new(get_ctx);

fn normalize(input: &[f32]) -> Vec<f32> {
    let magnitude = input
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        .sqrt();

    input.iter().map(|&val| val / magnitude).collect()
}

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let model = MODEL.as_ref().map_err(E::msg)?;
    let backend = BACKEND.as_ref().map_err(E::msg)?;

    let mut ctx = model.new_context(
        backend,
        LlamaContextParams::default()
            .with_flash_attention(true)
            .with_pooling_type(LlamaPoolingType::Last)
            .with_embeddings(true),
    )?;
    let n_ctx = ctx.n_ctx() as usize;
    let mut batch = LlamaBatch::new(n_ctx, 1);
    let tokens = model.str_to_token(&text.into(), AddBos::Always)?;

    batch.add_sequence(&tokens, 0, false)?;
    ctx.decode(&mut batch)?;
    let embedding = ctx.embeddings_seq_ith(0)?;
    Ok(normalize(embedding))
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
        let embedding_again = text_embedding(text).await?;
        assert_eq!(embedding, embedding_again);
        println!("Embedding: {:?}", embedding);
        assert_eq!(embedding.len(), 1024); // Assuming the model outputs 1024-dimensional embeddings
        let embedding_2 = text_embedding(text2).await?;
        assert!(embedding != embedding_2);
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
