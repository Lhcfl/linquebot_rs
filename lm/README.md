# LM

A small transformers language model for the bot.

## Inference

Simply run `cargo r --bin infer model_path` with desired prompt as stdin.
Here the model path is typically `data/model.mpk`.

## Training

First of all, export telegram chat messages to json format and copy the result.json to `data/result.json`.
Nothing else is needed, and only text messages with some simple markups are accepted.
Stickers and images would be filtered out and ignored.

Before starting training, some steps must be performed.
They can easily be done by running the corresponding binary with `cargo r --bin name`:

- Train the tokenizer (MUST be the first step): `train_tokenizer`;
- Translate the message json into datasets: `trans_dataset`;
- Run pretrain: `pretrain`.

Training can be continued by simply rerunning pretrain:
it would automatically read `data/model.mpk` to resume the model.
And therefore, also remember to remove the model if it's desired to retrain the model or model config changed.

## Changing the model

**Note**: it's better to remove the data dir and rerun the whole training process after changing the model.
Unless it's known what's being done.

The backend for training and inference (no other process need backend) is `lib.rs::DefaultBackend`.
Normally changing this after training won't affect inference,
but note that the default float precision may as well be provided as generic argument (like it's value in HEAD),
in which case the inference would be affected if the precision is changed.

The model config (layer num, hidden size, etc.) is provided in `impl Default for LmConfig` in `lib.rs`.
The default config is not clever (0.08b --- too small), and would definitely overfit on small groups/messages.
