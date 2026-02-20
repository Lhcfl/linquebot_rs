#![feature(read_array)]
#![feature(coroutines)]
#![feature(iter_from_coroutine)]
use core::f32;

use burn::{
    Tensor,
    config::Config,
    module::Module,
    nn::{
        Embedding, EmbeddingConfig, Linear, LinearConfig, RmsNorm, RmsNormConfig,
        loss::CrossEntropyLossConfig,
    },
    prelude::Backend,
    tensor::{
        Bool, DType, FloatDType, Int,
        activation::{silu, softmax},
        backend::AutodiffBackend,
    },
    train::{ClassificationOutput, InferenceStep, TrainOutput, TrainStep},
};

pub mod dataset;
pub mod read_messages;

pub const DELIM: &'static str = "<|delim|>";
pub type DefaultBackend = burn::backend::LibTorch<burn::tensor::bf16>;

#[derive(Config, Debug)]
pub struct LmConfig {
    pub hidden_size: usize,
    pub vocab_size: usize,
    pub intermediate_size: usize,
    pub num_layers: usize,
    pub num_attn_heads: usize,
    pub num_kv_heads: usize,
    pub max_seq_len: usize,
    pub rmsnorm_eps: f64,
    pub rope_theta: f64,
}

impl Default for LmConfig {
    fn default() -> Self {
        Self {
            hidden_size: 512,
            vocab_size: 16384,
            intermediate_size: 1536,
            num_layers: 15,
            num_attn_heads: 8,
            num_kv_heads: 4,
            max_seq_len: 1024,
            rmsnorm_eps: 1e-6,
            rope_theta: 10000.0,
        }
    }
}

impl LmConfig {
    pub fn head_dim(&self) -> usize {
        self.hidden_size / self.num_attn_heads
    }
}

/// Extra information that's not stored in the model.
/// Used in both generation and training.
#[derive(Debug, Clone)]
pub struct ExtraInfo<B: Backend> {
    pub k_cache: Option<Vec<Option<Tensor<B, 4>>>>,
    pub v_cache: Option<Vec<Option<Tensor<B, 4>>>>,
    pub position: usize,
    pub rope: Qwen3RoPE<B>,
    pub mask: Tensor<B, 2, Bool>,
    pub d_head: usize,
}

impl<B: Backend> ExtraInfo<B> {
    pub fn new(config: &LmConfig, need_cache: bool, device: &B::Device) -> Self {
        let (k_cache, v_cache) = if need_cache {
            (Some(vec![None; config.num_layers]), Some(vec![None; config.num_layers]))
        } else {
            (None, None)
        };
        let rope = Qwen3RoPE::new(config.head_dim(), config.rope_theta as f32, device);
        let mask = Tensor::tril_mask([config.max_seq_len, config.max_seq_len], 0, device);
        Self { k_cache, v_cache, position: 0, rope, mask, d_head: config.num_attn_heads }
    }

    /// Update cache after position, effectively concatenating the values after position
    pub fn update(
        &mut self,
        layer: usize,
        new_k: Tensor<B, 4>,
        new_v: Tensor<B, 4>,
    ) -> (Tensor<B, 4>, Tensor<B, 4>) {
        let (Some(k_cache_vec), Some(v_cache_vec)) = (&mut self.k_cache, &mut self.v_cache) else {
            return (new_k, new_v);
        };
        // Take, update, and put back
        let k_cache = if let Some(k_cache) = k_cache_vec[layer].take() {
            Tensor::cat(vec![k_cache, new_k], 2)
        } else {
            new_k
        };
        k_cache_vec[layer] = Some(k_cache.clone());
        let v_cache = if let Some(v_cache) = v_cache_vec[layer].take() {
            Tensor::cat(vec![v_cache, new_v], 2)
        } else {
            new_v
        };
        v_cache_vec[layer] = Some(v_cache.clone());
        (k_cache, v_cache)
    }

    pub fn update_pos(&mut self, seq_len: usize) {
        if self.k_cache.is_some() {
            self.position += seq_len;
        }
    }

    pub fn mk_casual_mask(
        &self,
        batch_size: usize,
        seq_len: usize,
        kv_seq_len: usize,
    ) -> Tensor<B, 4, Bool> {
        let max_len = self.mask.dims()[0];
        self.mask
            .clone()
            .slice([max_len - seq_len..max_len, max_len - kv_seq_len..max_len])
            .unsqueeze()
            .repeat_dim(0, batch_size)
            .repeat_dim(1, self.d_head)
    }
}
#[derive(Module, Debug)]
pub struct Qwen3RoPE<B: Backend> {
    pub inv_freq: Tensor<B, 1>,
}

impl<B: Backend> Qwen3RoPE<B> {
    pub fn new(head_dim: usize, theta: f32, device: &B::Device) -> Self {
        assert_eq!(head_dim % 2, 0, "head_dim must be even for RoPE");

        // inv_freq[i] = 1 / (theta ^ (2i / head_dim))
        let exponent = Tensor::<B, 1, Int>::arange_step(0..head_dim as i64, 2, device)
            .float()
            .cast(DType::F32)
            .div_scalar(head_dim as f32);
        let inv_freq = exponent.mul_scalar(theta.ln()).exp().recip().cast(DType::F32);

        Self { inv_freq }
    }

    pub fn apply(
        &self,
        q: Tensor<B, 4>,
        k: Tensor<B, 4>,
        start_pos: usize,
    ) -> (Tensor<B, 4>, Tensor<B, 4>) {
        let seq_len = q.dims()[2];
        let end_pos = start_pos + seq_len;
        let q_dtype = q.dtype();
        let half_dim = self.inv_freq.dims()[0];

        let positions = Tensor::<B, 1, Int>::arange(start_pos as i64..end_pos as i64, &q.device())
            .float()
            .cast(DType::F32)
            .reshape([seq_len, 1]);
        let inv_freq = self.inv_freq.clone().cast(DType::F32).reshape([1, half_dim]);
        let freqs: Tensor<B, 2> = positions.matmul(inv_freq);
        let emb: Tensor<B, 2> = Tensor::cat(vec![freqs.clone(), freqs], 1);

        let cos = emb.clone().cos().unsqueeze_dim::<3>(0).unsqueeze_dim::<4>(0);
        let sin = emb.sin().unsqueeze_dim::<3>(0).unsqueeze_dim::<4>(0);

        let q = q.cast(DType::F32);
        let k = k.cast(DType::F32);
        let q_embed = (q.clone() * cos.clone() + Self::rotate_half(q) * sin.clone()).cast(q_dtype);
        let k_embed = (k.clone() * cos + Self::rotate_half(k) * sin).cast(q_dtype);
        (q_embed, k_embed)
    }

    fn rotate_half(x: Tensor<B, 4>) -> Tensor<B, 4> {
        let [batch_size, num_heads, seq_len, d] = x.dims();
        let x1 = x.clone().slice([0..batch_size, 0..num_heads, 0..seq_len, 0..d / 2]);
        let x2 = x.slice([0..batch_size, 0..num_heads, 0..seq_len, d / 2..d]);
        Tensor::cat(vec![x2.mul_scalar(-1.0), x1], 3)
    }
}

#[derive(Module, Debug)]
pub struct LmModel<B: Backend> {
    pub embed: Embedding<B>,
    pub layers: Vec<Decoder<B>>,
    pub norm: RmsNorm<B>,
    pub lm_head: Linear<B>,
}

#[derive(Module, Debug)]
pub struct Decoder<B: Backend> {
    pub input_norm: RmsNorm<B>,
    pub attn: SelfAttn<B>,
    pub attn_norm: RmsNorm<B>,
    pub mlp: MLP<B>,
}

#[derive(Module, Debug)]
pub struct SelfAttn<B: Backend> {
    pub q_proj: Linear<B>,
    pub k_proj: Linear<B>,
    pub v_proj: Linear<B>,
    pub o_proj: Linear<B>,
    pub q_norm: RmsNorm<B>,
    pub k_norm: RmsNorm<B>,
    pub num_q_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
}

#[derive(Module, Debug)]
pub struct MLP<B: Backend> {
    pub gate: Linear<B>,
    pub up: Linear<B>,
    pub down: Linear<B>,
}

impl<B: Backend> LmModel<B> {
    pub fn new(config: &LmConfig, device: &B::Device) -> Self {
        let embed = EmbeddingConfig::new(config.vocab_size, config.hidden_size).init(device);
        let layers = (0..config.num_layers).map(|_| Decoder::new(config, device)).collect();
        let norm =
            RmsNormConfig::new(config.hidden_size).with_epsilon(config.rmsnorm_eps).init(device);
        let lm_head =
            LinearConfig::new(config.hidden_size, config.vocab_size).with_bias(false).init(device);
        Self { embed, layers, norm, lm_head }
    }
    pub fn forward(&self, ids: Tensor<B, 2, Int>, info: &mut ExtraInfo<B>) -> Tensor<B, 3> {
        //let x = self.embed.forward(ids);
        let [bs, seq_len] = ids.dims();
        let [vocab, hs] = self.embed.weight.dims();
        let ids = ids.reshape([-1]);
        log::info!("vocab {vocab}, ids: {:?}", ids.to_data().as_slice::<i64>().unwrap());
        let x = self.embed.weight.val().select(0, ids).reshape([bs, seq_len, hs]);
        let x =
            self.layers.iter().enumerate().fold(x, |x, (layer, sub)| sub.forward(x, layer, info));
        info.update_pos(x.dims()[1]);
        let x = self.norm.forward(x);
        self.lm_head.forward(x)
    }

    pub fn inspect(&self) {
    }
}

impl<B: Backend> Decoder<B> {
    pub fn new(config: &LmConfig, device: &B::Device) -> Self {
        let input_norm =
            RmsNormConfig::new(config.hidden_size).with_epsilon(config.rmsnorm_eps).init(device);
        let attn = SelfAttn::new(config, device);
        let attn_norm =
            RmsNormConfig::new(config.hidden_size).with_epsilon(config.rmsnorm_eps).init(device);
        let mlp = MLP::new(config, device);
        Self { input_norm, attn, attn_norm, mlp }
    }
    pub fn forward(&self, x: Tensor<B, 3>, layer: usize, info: &mut ExtraInfo<B>) -> Tensor<B, 3> {
        let residual = x.clone();
        let x = self.input_norm.forward(x);
        let x = self.attn.forward(x, layer, info) + residual;

        let residual = x.clone();
        let x = self.attn_norm.forward(x);
        self.mlp.forward(x) + residual
    }
}

impl<B: Backend> SelfAttn<B> {
    pub fn new(config: &LmConfig, device: &B::Device) -> Self {
        let num_q_heads = config.num_attn_heads;
        let num_kv_heads = config.num_kv_heads;
        let head_dim = config.head_dim();
        let q_proj = LinearConfig::new(config.hidden_size, num_q_heads * head_dim)
            .with_bias(false)
            .init(device);
        let k_proj = LinearConfig::new(config.hidden_size, num_kv_heads * head_dim)
            .with_bias(false)
            .init(device);
        let v_proj = LinearConfig::new(config.hidden_size, num_kv_heads * head_dim)
            .with_bias(false)
            .init(device);
        let o_proj = LinearConfig::new(num_q_heads * head_dim, config.hidden_size)
            .with_bias(false)
            .init(device);
        let q_norm = RmsNormConfig::new(head_dim).with_epsilon(config.rmsnorm_eps).init(device);
        let k_norm = RmsNormConfig::new(head_dim).with_epsilon(config.rmsnorm_eps).init(device);
        Self { q_proj, k_proj, v_proj, o_proj, q_norm, k_norm, num_q_heads, num_kv_heads, head_dim }
    }

    pub fn forward(&self, x: Tensor<B, 3>, layer: usize, info: &mut ExtraInfo<B>) -> Tensor<B, 3> {
        let [batch_size, seq_len, _hidden] = x.dims();
        let q = self.q_proj.forward(x.clone());
        let k = self.k_proj.forward(x.clone());
        let v = self.v_proj.forward(x.clone());

        // Reshape to [batch size, Dq/Dkv, seq len, Dh]
        let q = q.reshape([0, 0, self.num_q_heads, self.head_dim]).swap_dims(1, 2);
        let k = k.reshape([0, 0, self.num_kv_heads, self.head_dim]).swap_dims(1, 2);
        let v = v.reshape([0, 0, self.num_kv_heads, self.head_dim]).swap_dims(1, 2);

        let (q, k) = (self.q_norm.forward(q), self.k_norm.forward(k));
        let (q, k) = info.rope.apply(q, k, info.position);
        let (k, v) = info.update(layer, k, v);

        let (k, v) = if self.num_q_heads != self.num_kv_heads {
            let final_shape: [isize; _] =
                [0, self.num_q_heads as isize, -1, self.head_dim as isize];
            let nrep = self.num_q_heads / self.num_kv_heads;
            (
                k.unsqueeze_dim::<5>(2).repeat_dim(2, nrep).reshape(final_shape),
                v.unsqueeze_dim::<5>(2).repeat_dim(2, nrep).reshape(final_shape),
            )
        } else {
            (k, v)
        };

        let dtype = q.dtype();
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let mask = info.mk_casual_mask(batch_size, seq_len, k.dims()[2]);
        let attn = q.matmul(k.swap_dims(2, 3)) * scale;
        // casual mask to avoid attend to later tokens
        let attn = attn.mask_fill(mask, -1e9);
        let attn = softmax(attn.cast(FloatDType::F32), 3).cast(dtype);
        let attn = attn.matmul(v);

        // Back to [bs, len, hidden]
        let attn = attn.swap_dims(1, 2).reshape([0, 0, -1]);
        self.o_proj.forward(attn)
    }
}

impl<B: Backend> MLP<B> {
    pub fn new(config: &LmConfig, device: &B::Device) -> Self {
        let gate = LinearConfig::new(config.hidden_size, config.intermediate_size)
            .with_bias(false)
            .init(device);
        let up = LinearConfig::new(config.hidden_size, config.intermediate_size)
            .with_bias(false)
            .init(device);
        let down = LinearConfig::new(config.intermediate_size, config.hidden_size)
            .with_bias(false)
            .init(device);
        Self { gate, up, down }
    }
    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let gated = silu(self.gate.forward(x.clone())) * self.up.forward(x);
        self.down.forward(gated)
    }
}

#[derive(Debug, Clone)]
pub struct LmBatch<B: Backend> {
    pub input: Tensor<B, 2, Int>,
    pub target: Tensor<B, 2, Int>,
    pub info: ExtraInfo<B>,
}

impl<B: AutodiffBackend> TrainStep for LmModel<B> {
    type Input = LmBatch<B>;
    type Output = burn::train::ClassificationOutput<B>;
    fn step(&self, item: Self::Input) -> burn::train::TrainOutput<Self::Output> {
        let item = InferenceStep::step(self, item);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> InferenceStep for LmModel<B> {
    type Input = LmBatch<B>;
    type Output = burn::train::ClassificationOutput<B>;
    fn step(&self, mut item: Self::Input) -> Self::Output {
        let output = self.forward(item.input.clone(), &mut item.info);
        let target = item.target.reshape([-1]);
        let output = output.reshape([target.dims()[0] as isize, -1]);
        let loss = CrossEntropyLossConfig::new()
            .init(&output.device())
            .forward(output.clone(), target.clone());
        ClassificationOutput::new(loss, output, target)
    }
}
