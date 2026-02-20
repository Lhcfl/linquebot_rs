use lm::{dataset::ChatWriter, read_messages::MsgBundle};
use rand::RngExt;
use tokenizers::Tokenizer;

fn main() -> anyhow::Result<()> {
    let msgs = MsgBundle::from_file("data/result.json")?;
    let msgs = msgs.iter()?;
    let mut train_db = ChatWriter::new(
        "data/train_dataset.bin",
        Tokenizer::from_file("data/tokenizer.json").map_err(anyhow::Error::from_boxed)?,
    )?;
    let mut valid_db = ChatWriter::new(
        "data/validate_dataset.bin",
        Tokenizer::from_file("data/tokenizer.json").map_err(anyhow::Error::from_boxed)?,
    )?;
    let mut rng = rand::rng();
    let mut valid_len = 0;
    for (i, it) in msgs.enumerate() {
        if rng.random::<f64>() < 0.0025 || valid_len > 0 {
            valid_db.add(&it)?;
            if valid_len == 0 {
                valid_len = 4;
            } else {
                valid_len -= 1;
            }
        } else {
            train_db.add(&it)?;
        }
        whole_db.add(&it)?;
    }
    Ok(())
}
