use std::any::Any;

use burn::train::metric::Adaptor;
use lm::LmConfig;
use tokenizers::{
    AddedToken, TokenizerBuilder,
    decoders::byte_level::ByteLevel,
    models::bpe::{BPE, BpeTrainerBuilder},
    normalizers::NFC,
};

fn main() -> anyhow::Result<()> {
    let conf_path = std::path::Path::new("data/config.json");
    let conf: LmConfig = if conf_path.exists() {
        serde_json::from_reader(std::fs::File::open(conf_path)?)?
    } else {
        let res = LmConfig::default();
        serde_json::to_writer(std::fs::File::create(conf_path)?, &res)?;
        res
    };
    let mut tokenizer = TokenizerBuilder::new()
        .with_model(BPE::default())
        .with_normalizer(Some(NFC::default()))
        .with_pre_tokenizer(Some(ByteLevel::default()))
        .with_post_processor(Some(ByteLevel::default()))
        .with_decoder(Some(ByteLevel::default()))
        .build()
        .map_err(anyhow::Error::from_boxed)?;
    let mut trainer = BpeTrainerBuilder::new()
        .show_progress(true)
        .vocab_size(conf.vocab_size)
        .min_frequency(2)
        .special_tokens(vec![AddedToken::from(lm::DELIM, true)])
        .build();
    let msgs = lm::read_messages::MsgBundle::from_file(std::path::Path::new("data/result.json"))?;
    tokenizer.train(&mut trainer, msgs.iter()?).map_err(anyhow::Error::from_boxed)?;
    tokenizer.save("data/tokenizer.json", true).map_err(anyhow::Error::from_boxed)?;
    Ok(())
}
