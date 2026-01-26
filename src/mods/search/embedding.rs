use anyhow::{Error as E, Result};
use hf_hub::{Repo, RepoType, api::sync::Api};
use llama_cpp_2::{
    LogOptions,
    context::params::{LlamaContextParams, LlamaPoolingType},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel},
    send_logs_to_tracing,
};
use std::sync::LazyLock;
use tokio::sync::oneshot;

static MODEL_ID: &str = "Qwen/Qwen3-Embedding-0.6B-GGUF";
static REVISION: &str = "main";

fn get_model(backend: &LlamaBackend) -> Result<LlamaModel> {
    send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));
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

fn normalize(input: &[f32]) -> Vec<f32> {
    let magnitude = input
        .iter()
        .fold(0.0, |acc, &val| val.mul_add(val, acc))
        .sqrt();

    input.iter().map(|&val| val / magnitude).collect()
}

#[derive(Debug)]
enum WorkerCommand {
    Text {
        text: String,
        response: oneshot::Sender<Result<Vec<f32>>>,
    },
}

#[derive(Debug, Clone)]
pub struct EmbeddingWorkerHandle {
    command_sender: tokio::sync::mpsc::Sender<WorkerCommand>,
}

pub struct EmbeddingWorker {
    model: LlamaModel,
    backend: LlamaBackend,
    command_receiver: tokio::sync::mpsc::Receiver<WorkerCommand>,
}

impl EmbeddingWorker {
    pub fn new() -> Result<(Self, EmbeddingWorkerHandle)> {
        let backend = get_backend()?;
        let model = get_model(&backend)?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(32);
        let handle = EmbeddingWorkerHandle { command_sender };
        Ok((
            Self {
                model,
                backend,
                command_receiver,
            },
            handle,
        ))
    }

    fn do_embedding(
        &self,
        context: &mut llama_cpp_2::context::LlamaContext<'_>,
        text: &str,
    ) -> Result<Vec<f32>> {
        context.clear_kv_cache();
        let n_ctx = context.n_ctx() as usize;
        let mut batch = LlamaBatch::new(n_ctx, 1);
        let tokens = self.model.str_to_token(text, AddBos::Always)?;
        batch.add_sequence(&tokens, 0, false)?;
        context.decode(&mut batch)?;
        let embedding = context.embeddings_seq_ith(0)?;
        // batch.clear();
        Ok(normalize(embedding))
    }

    pub fn run(mut self) {
        let mut context = self
            .model
            .new_context(
                &self.backend,
                LlamaContextParams::default()
                    .with_flash_attention_policy(-1) // auto
                    .with_pooling_type(LlamaPoolingType::Last)
                    .with_embeddings(true),
            )
            .expect("Failed to create context");

        while let Some(command) = self.command_receiver.blocking_recv() {
            match command {
                WorkerCommand::Text { text, response } => {
                    let result = self.do_embedding(&mut context, &text);
                    response.send(result).ok();
                }
            }
        }
    }
}

static WORKER: LazyLock<Result<EmbeddingWorkerHandle>> = LazyLock::new(|| {
    let (worker, handle) = EmbeddingWorker::new()?;
    std::thread::spawn(|| {
        worker.run();
    });
    Ok(handle)
});

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let worker = WORKER.as_ref().map_err(E::msg)?;
    let (tx, rx) = oneshot::channel();
    worker
        .command_sender
        .send(WorkerCommand::Text {
            text: text.into(),
            response: tx,
        })
        .await?;
    let res = rx.await??;
    Ok(res)
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
