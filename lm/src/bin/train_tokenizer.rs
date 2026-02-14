use tokenizers::{
    TokenizerBuilder,
    decoders::byte_level::ByteLevel,
    models::bpe::{BPE, BpeTrainerBuilder},
};

fn main() -> anyhow::Result<()> {
    let mut tokenizer = TokenizerBuilder::new()
        .with_model(BPE::default())
        .with_pre_tokenizer(Some(ByteLevel::default()))
        .build()?;
    let mut trainer = BpeTrainerBuilder::new()
        .show_progress(true)
        .vocab_size(80000)
        .min_frequency(2)
        .special_tokens(vec!["<|conversation-delim|>".into()])
        .build();
    tokenizer.train(&mut trainer, ["conversation.txt"])?;
    tokenizer.save("tokenizer.json", true)?;
}
