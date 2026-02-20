#![recursion_limit = "256"]
use std::{marker::PhantomData, path::PathBuf};

use burn::{
    Tensor,
    backend::Autodiff,
    config::Config,
    data::{
        dataloader::{DataLoaderBuilder, batcher::Batcher},
        dataset::transform::{SamplerDataset, SamplerDatasetOptions},
    },
    module::Module,
    optim::AdamWConfig,
    prelude::Backend,
    record::{FullPrecisionSettings, NamedMpkFileRecorder},
    tensor::{Int, backend::AutodiffBackend},
    train::{
        ClassificationOutput, Learner, SupervisedTraining,
        metric::{
            Adaptor, LossMetric, Metric, MetricName, Numeric,
            state::{FormatOptions, NumericMetricState},
        },
    },
};
use lm::{
    DefaultBackend, ExtraInfo, LmBatch, LmConfig, LmModel,
    dataset::{ChatFile, SeqLenWrapper},
};
use tokenizers::Tokenizer;

#[derive(Debug, Clone)]
pub struct LmBatcher {
    pub model: LmConfig,
    pub seq_len: usize,
    pub tokenizer: Tokenizer,
}

impl<B: Backend> Batcher<B, Vec<u32>, LmBatch<B>> for LmBatcher {
    fn batch(&self, xs: Vec<Vec<u32>>, device: &B::Device) -> LmBatch<B> {
        let xs = xs.into_iter().flatten().collect::<Vec<u32>>();
        let ns = xs.len();
        let raw = xs.clone();
        let xs = Tensor::<B, 1, Int>::from_ints(xs.as_slice(), device);
        let lpad = xs
            .clone()
            .slice([0..ns / self.seq_len * self.seq_len])
            .reshape([-1, self.seq_len as isize]);
        let rpad = xs.slice([ns % self.seq_len..ns]).reshape([-1, self.seq_len as isize]);
        let xs = Tensor::cat(vec![lpad, rpad], 0);
        log::info!(
            "raw ids: {raw:?} data {:?}\ntokenized: {:?}",
            xs.to_data().as_slice::<i64>(),
            xs.to_data()
                .iter()
                .map(|c| self.tokenizer.decode(&[c], true).unwrap())
                .collect::<Vec<_>>()
        );
        let input = xs.clone().slice_dim(1, ..xs.dims()[1] - 1);
        let target = xs.slice_dim(1, 1..);
        LmBatch { input, target, info: ExtraInfo::new(&self.model, false, device) }
    }
}

pub struct LossInput<B: Backend>(Tensor<B, 1>);

impl<B: Backend> Adaptor<LossInput<B>> for ClassificationOutput<B> {
    fn adapt(&self) -> LossInput<B> {
        LossInput(self.loss.clone())
    }
}

#[derive(Clone)]
pub struct LogLossMetric<B: Backend> {
    name: MetricName,
    state: NumericMetricState,
    _b: PhantomData<B>,
}

impl<B: Backend> LogLossMetric<B> {
    pub fn new() -> Self {
        Self {
            name: MetricName::new("Log Loss".into()),
            state: NumericMetricState::new(),
            _b: PhantomData,
        }
    }
}

impl<B: Backend> Metric for LogLossMetric<B> {
    type Input = LossInput<B>;

    fn name(&self) -> MetricName {
        self.name.clone()
    }

    fn update(
        &mut self,
        item: &Self::Input,
        _metadata: &burn::train::metric::MetricMetadata,
    ) -> burn::train::metric::SerializedEntry {
        let bs = item.0.dims()[0];
        let val = item.0.clone().mean().log().into_data().iter().next().expect("mean data");
        self.state.update(val, bs, FormatOptions::new(self.name()).precision(3))
    }

    fn clear(&mut self) {
        self.state.reset();
    }
}

impl<B: Backend> Numeric for LogLossMetric<B> {
    fn value(&self) -> burn::train::metric::NumericEntry {
        self.state.current_value()
    }

    fn running_value(&self) -> burn::train::metric::NumericEntry {
        self.state.running_value()
    }
}

#[derive(Config, Debug)]
pub struct TrainConfig {
    pub model: LmConfig,
    pub optimizer: AdamWConfig,
    #[config(default = 19260817)]
    pub seed: u64,
    #[config(default = 50)]
    pub num_epochs: usize,
    #[config(default = 16)]
    pub batch_size: usize,
    #[config(default = 128)]
    pub seq_len: usize,
    #[config(default = 2)]
    pub n_workers: usize,
    #[config(default = 3.0e-4)]
    pub learning_rate: f64,
}

pub fn train<B: AutodiffBackend>(
    data_dir: PathBuf,
    config: TrainConfig,
    device: &B::Device,
) -> anyhow::Result<()> {
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

    config.save("train_config.json")?;
    B::seed(device, config.seed);

    let train_data =
        SeqLenWrapper::new(ChatFile::new(data_dir.join("train_dataset.bin"))?, config.seq_len + 1);
    let train_data =
        SamplerDataset::new(train_data, SamplerDatasetOptions::default().with_size_ratio(0.01));
    let val_data = SeqLenWrapper::new(
        ChatFile::new(data_dir.join("validate_dataset.bin"))?,
        config.seq_len + 1,
    );

    let mpk_recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
    let batcher = LmBatcher {
        model: config.model.clone(),
        seq_len: config.seq_len,
        tokenizer: Tokenizer::from_file(data_dir.join("tokenizer.json")).unwrap(),
    };
    let dataloader_train = DataLoaderBuilder::<B, _, _>::new(batcher.clone())
        .batch_size(config.batch_size)
        .num_workers(config.n_workers)
        .shuffle(config.seed)
        .build(train_data);
    let dataloader_valid = DataLoaderBuilder::<B::InnerBackend, _, _>::new(batcher)
        .batch_size(config.batch_size)
        .num_workers(config.n_workers)
        .build(val_data);
    let training = SupervisedTraining::new(data_dir.clone(), dataloader_train, dataloader_valid)
        .num_epochs(config.num_epochs)
        .metrics((LogLossMetric::new(), LossMetric::new()))
        //.with_checkpointing_strategy(KeepLastNCheckpoints::new(5))
        //.with_file_checkpointer(mpk_recorder.clone())
        .summary();
    //let training = training.renderer(CliMetricsRenderer::new());

    let model_path = data_dir.join("model.mpk");
    let model = LmModel::new(&config.model, device);
    let model = if model_path.exists() {
        model.load_file(model_path.clone(), &mpk_recorder, device)?
    } else {
        model
    };

    let learner = Learner::new(model, config.optimizer.init(), config.learning_rate);
    let res = training.launch(learner);
    res.model.save_file(model_path, &mpk_recorder)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let lm = LmConfig::default();
    let adam = AdamWConfig::new()
        .with_grad_clipping(Some(burn::grad_clipping::GradientClippingConfig::Norm(1.)));
    let device = Default::default();
    train::<Autodiff<DefaultBackend>>("data/".into(), TrainConfig::new(lm, adam), &device)?;
    Ok(())
}
