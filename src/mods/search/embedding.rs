use anyhow::{Error as E, Result};
use hf_hub::{api::sync::Api, Repo, RepoType};
use ndarray::Ix1;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use std::{
    fs::{self, File},
    io::Read,
    sync::LazyLock,
};
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

fn read_onnx() -> Result<Vec<u8>> {
    let repo = Repo::with_revision(MODEL_ID.to_string(), RepoType::Model, REVISION.to_string());
    let api = Api::new()?;
    let api = api.repo(repo.to_owned());
    let onnx_filename = api.get("onnx/model_uint8.onnx")?;
    let mut f = File::open(&onnx_filename)?;
    let metadata = fs::metadata(&onnx_filename)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer)?;
    Ok(buffer)
}

static SESSION_BUF: LazyLock<Result<Vec<u8>>> = LazyLock::new(read_onnx);

fn get_session() -> Result<Session> {
    let session_buf = SESSION_BUF.as_deref().map_err(E::msg)?;
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_memory(session_buf)?;
    Ok(session)
}

static TOKENIZER: LazyLock<Result<Tokenizer>> = LazyLock::new(get_tokenizer);

pub fn text_embedding(text: impl Into<String>) -> Result<Vec<f32>> {
    let tokenizer = TOKENIZER.as_ref().map_err(E::msg)?;
    let mut session = get_session()?;
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
    let outputs = session.run(ort::inputs![tokens, attention_mask])?;
    let embeddings = outputs[1]
        .try_extract_array::<f32>()?
        .squeeze()
        .into_dimensionality::<Ix1>()?
        .to_vec();
    Ok(embeddings)
}
