use log::warn;
use msg_context::Context;
use msg_context::TaskContext;
use rand::Rng;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;

async fn reply(ctx: TaskContext, text: &str) {
    if let Err(err) = ctx.reply(text).send().await {
        warn!("Failed to send reply: {}", err.to_string());
    }
}

pub fn dice(ctx: &mut Context, message: &Message) -> Consumption {
    use crate::utils::pattern::*;

    let text = ctx.cmd?.content;
    let Some(from) = message.from.clone() else {
        warn!("No reply target.");
        return Consumption::Stop;
    };

    let ctx = ctx.task();

    let Some((_, (x, _, y))) = (
        of_pred(|c| c.is_ascii_digit()),
        "d",
        of_pred(|c| c.is_ascii_digit()),
    )
        .check_pattern(text)
    else {
        return Consumption::StopWith(Box::pin(reply(
            ctx,
            "参数必须是 xdy 的格式，其中 x 和 y 是正整数",
        )));
    };

    if x.is_empty() || y.is_empty() {
        return Consumption::StopWith(Box::pin(reply(
            ctx,
            "参数必须是 xdy 的格式，其中 x 和 y 是正整数",
        )));
    }

    let Ok(x) = x.parse::<u16>() else {
        return Consumption::StopWith(Box::pin(reply(ctx, "提供的 x 太大了！")));
    };

    let Ok(y) = y.parse::<u32>() else {
        return Consumption::StopWith(Box::pin(reply(ctx, "提供的 y 太大了！")));
    };

    if x > 500 {
        return Consumption::StopWith(Box::pin(reply(ctx, "提供的 x 太大了！")));
    }

    Consumption::StopWith(Box::pin(async move {
        let results = (0..x)
            .map(|_| rand::thread_rng().gen_range(1..=y as u64))
            .collect::<Vec<_>>();

        // x 个 u32 的和肯定不会超过 u64，可以放心不会 panic
        let sum: u64 = results.iter().sum();
        let str = format!("{} 掷出了：{}: {:?}", from.full_name(), sum, results);
        if str.len() >= 4095 {
            reply(ctx, "你的 xdy 太大了，超过了能发送的长度").await;
        } else {
            reply(ctx, &str).await;
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDesctiption {
        name: "dice",
        description: "`xdy` 抛掷 x 个 y 面的骰子",
        description_detailed: Some(concat!(
            "使用 `/dice xdy` 的格式，抛掷 x 个 y 面的骰子。\n",
            "结果返回 `骰子总和: [每个骰子点数...]`\n",
            "注意 x 最大不能超过 500，y 最大不能超过 4294967295"
        )),
    }),
    task: dice,
};
