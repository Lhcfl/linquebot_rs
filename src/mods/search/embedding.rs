use anyhow::{Error as E, Result};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use ndarray::Ix1;
use ort::{
    session::{builder::GraphOptimizationLevel, RunOptions, Session},
    value::TensorRef,
};
use tokenizers::Tokenizer;

static MODEL_ID: &str = "Snowflake/snowflake-arctic-embed-l-v2.0";
static REVISION: &str = "main";

async fn get_tokenizer() -> Result<Tokenizer> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo.to_owned());
    let tokenizer_filename = api.get("tokenizer.json").await?;
    let tokenizer = Tokenizer::from_file(tokenizer_filename)
        .map_err(E::msg)?
        .with_padding(None)
        .with_truncation(None)
        .map_err(E::msg)?
        .to_owned()
        .into();
    Ok(tokenizer)
}

async fn get_session() -> Result<Session> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo.to_owned());
    let onnx_filename = api.get("onnx/model_uint8.onnx").await?;
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(onnx_filename)?;
    Ok(session)
}

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let tokenizer = get_tokenizer().await?;
    let mut session = get_session().await?;
    let encoding = tokenizer.encode(text.into(), true).map_err(E::msg)?;
    let tokens = encoding
        .get_ids()
        .iter()
        .map(|&t| t as i64)
        .collect::<Vec<i64>>();
    let attention_mask = encoding
        .get_attention_mask()
        .iter()
        .map(|&m| m as i64)
        .collect::<Vec<i64>>();
    let tokens = TensorRef::from_array_view(([1, encoding.len()], &*tokens))?;
    let attention_mask = TensorRef::from_array_view(([1, encoding.len()], &*attention_mask))?;
    let options = RunOptions::new()?;
    let outputs = session
        .run_async(ort::inputs![tokens, attention_mask], &options)?
        .await?;
    let embeddings = outputs[1]
        .try_extract_array::<f32>()?
        .squeeze()
        .into_dimensionality::<Ix1>()?
        .to_vec();
    Ok(embeddings)
}
