use std::collections::HashMap;

use candle_onnx::onnx::ModelProto;

use anyhow::{Error as E, Result};
use candle_core::{Device, Tensor};
use candle_transformers::models::stable_diffusion::attention;
use hf_hub::{api::sync::Api, Repo, RepoType};
use log::info;
use tokenizers::Tokenizer;

fn build_model_and_tokenizer() -> Result<(ModelProto, Tokenizer)> {
    let model_id = "Snowflake/snowflake-arctic-embed-l-v2.0".to_string();
    let revision = "main".to_string();

    let repo = Repo::with_revision(model_id, RepoType::Model, revision);
    let (onnx_filename, tokenizer_filename) = {
        let api = Api::new()?;
        let api = api.repo(repo);
        let tokenizer = api.get("tokenizer.json")?;
        let onnx = api.get("onnx/model_uint8.onnx")?;
        (onnx, tokenizer)
    };

    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

    let model = candle_onnx::read_file(onnx_filename)?;
    Ok((model, tokenizer))
}

fn embed(text: impl Into<String>) -> Result<HashMap<std::string::String, candle_core::Tensor>> {
    let start = std::time::Instant::now();

    let (model, mut tokenizer) = build_model_and_tokenizer()?;
    let device = &Device::Cpu;

    let tokenizer = tokenizer
        .with_padding(None)
        .with_truncation(None)
        .map_err(E::msg)?;
    let encoding = tokenizer.encode(text.into(), true).map_err(E::msg)?;
    let tokens = encoding
        .get_ids()
        .iter()
        .map(|&t| t as i64)
        .collect::<Vec<i64>>();
    let token_ids = Tensor::new(&tokens[..], device)?.unsqueeze(0)?;
    let token_type_ids = token_ids.zeros_like()?;
    let attention_mask = token_ids.ones_like()?;

    println!("Loaded and encoded {:?}", start.elapsed());
    let mut inputs = HashMap::new();
    inputs.insert("input_ids".to_string(), token_ids);
    inputs.insert("token_type_ids".to_string(), token_type_ids);
    inputs.insert("attention_mask".to_string(), attention_mask);
    Ok(candle_onnx::simple_eval(&model, inputs).unwrap())
}

pub fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}

pub async fn text_embedding(text: impl Into<String>) -> Result<Vec<f64>> {
    let result = embed(text);
    info!("Embedding result: {:?}", result);

    Ok(vec![0.0; 768]) // Placeholder for actual embedding logic
}
