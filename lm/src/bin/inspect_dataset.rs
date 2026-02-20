use anyhow::Context;
use burn::data::dataset::Dataset;
use lm::dataset::{ChatFile, SeqLenWrapper};
use tokenizers::Tokenizer;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    let data = ChatFile::new(args.next().context("dataset in arg")?)?;
    let data = SeqLenWrapper::new(data, 128);
    let tokenizer =
        Tokenizer::from_file("data/tokenizer.json").map_err(anyhow::Error::from_boxed)?;
    println!("dataset len {}", data.len());
    for i in 0..=data.len() {
        let Some(it) = data.get(i) else {
            println!("(Empty at #{i})");
            continue;
        };
        println!("#{i}: {it:?} => '{}'", tokenizer.decode(&it, false).map_err(anyhow::Error::from_boxed)?);
    }
    Ok(())
}
