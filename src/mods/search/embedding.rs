use anyhow::{Error as E, Result};
use hf_hub::{api::sync::Api, Repo, RepoType};
use ndarray::{Array1, Axis, Ix1};
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::sync::LazyLock;
use tokenizers::Tokenizer;

static MODEL_ID: &str = "Snowflake/snowflake-arctic-embed-l-v2.0";
static REVISION: &str = "main";

fn get_tokenizer() -> Result<Tokenizer> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo.to_owned());
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

fn get_session() -> Result<Session> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo.to_owned());
    let onnx_filename = api.get("onnx/model_uint8.onnx")?;
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(onnx_filename)?;
    Ok(session)
}

static TOKENIZER: LazyLock<Result<Tokenizer>> = LazyLock::new(get_tokenizer);
static SESSION: LazyLock<Result<Session>> = LazyLock::new(get_session);

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let tokenizer = TOKENIZER.as_ref().map_err(E::msg)?;
    let session = SESSION.as_ref().map_err(E::msg)?;
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
    let tokens = Array1::from_vec(tokens).insert_axis(Axis(0));
    let attention_mask = Array1::from_vec(attention_mask).insert_axis(Axis(0));
    let outputs = session.run(ort::inputs![tokens, attention_mask]?)?;
    let embeddings = outputs[1]
        .try_extract_tensor()?
        .squeeze()
        .into_dimensionality::<Ix1>()?
        .to_vec();
    Ok(embeddings)
}
