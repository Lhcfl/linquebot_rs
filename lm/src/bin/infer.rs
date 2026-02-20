#![recursion_limit = "256"]
use std::io::{Read, Write};

use burn::{
    config::Config,
    module::Module,
    record::{FullPrecisionSettings, NamedMpkFileRecorder},
    tensor::{Int, Tensor, activation::softmax},
};
use lm::{DefaultBackend, ExtraInfo, LmConfig, LmModel};
use rand::RngExt;
use tokenizers::Tokenizer;

fn main() -> anyhow::Result<()> {
    color_backtrace::BacktracePrinter::new()
        .strip_function_hash(true)
        .add_frame_filter(Box::new(|frames| {
            let crate_path = std::path::Path::new(file!()).canonicalize().unwrap();
            let crate_path = crate_path.parent().unwrap().parent().unwrap();
            frames.retain(|f| {
                f.filename.as_ref().is_some_and(|f| {
                    f.canonicalize().ok().is_some_and(|f| f.starts_with(crate_path))
                })
            });
        }))
        .install(color_backtrace::default_output_stream());

    let mut args = std::env::args();
    args.next();
    let model_path = args.next().expect("model path argument required");

    let config = LmConfig::load("data/config.json")?;
    let tokenizer =
        Tokenizer::from_file("data/tokenizer.json").map_err(anyhow::Error::from_boxed)?;

    let device = Default::default();
    let model: LmModel<DefaultBackend> = LmModel::new(&config, &device);
    let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
    let model = model.load_file(model_path, &recorder, &device)?;

    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt)?;
    let prompt = prompt.trim();
    println!("prompt: '{prompt}'");

    let tokens = tokenizer.encode(prompt, true).map_err(anyhow::Error::from_boxed)?;
    let tokens = tokens.get_ids();
    let mut info = ExtraInfo::new(&config, true, &device);
    let mut input =
        Tensor::<_, 1, Int>::from_ints(tokens, &device).reshape([1, tokens.len() as isize]);

    let mut rng = rand::rng();
    for _ in 0..config.max_seq_len {
        let output = softmax(model.forward(input, &mut info), 2);
        let seq_len = output.dims()[1];
        let (prob, idx) = output
            .slice_dim(1, seq_len - 1..)
            .reshape([config.vocab_size as isize])
            .topk_with_indices(10, 0);
        /*println!(
            "{:?}",
            idx.clone()
                .to_data()
                .iter()
                .map(|i| (i, tokenizer.decode(&[i], true).unwrap()))
                .collect::<Vec<_>>()
        );*/
        let prob = prob.slice_dim(0, 0..10);
        let prob = (prob.clone().div(prob.sum_dim(0))).cumsum(0);
        let mask = prob.greater_elem(rng.random::<f32>()).int().argmax(0).reshape([-1]);
        let next = idx.select(0, mask).reshape([1, 1]);

        input = next.clone();

        let next = next.into_data().iter::<u32>().next().expect("token");
        if next == 0 {
            println!("<|delim|>");
        } else {
            print!("{}", tokenizer.decode(&[next], true).map_err(anyhow::Error::from_boxed)?);
        }
        std::io::stdout().flush()?;
        if next == 0 && rng.random::<f32>() < 0.3 {
            break;
        }
    }

    Ok(())
}
