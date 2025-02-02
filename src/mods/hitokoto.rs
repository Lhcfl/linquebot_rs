//! 一言

use log::trace;
use log::warn;
use msg_context::Context;
use serde::Deserialize;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::*;
use crate::Consumption;

#[derive(Deserialize, Debug)]
struct Hitokoto {
    hitokoto: String,
    from: String,
}

async fn get_hitokoto(args: &str) -> Hitokoto {
    let res: Result<_, reqwest::Error> = try {
        reqwest::get(format!("https://v1.hitokoto.cn/?c={}", args))
            .await?
            .json::<Hitokoto>()
            .await?
    };

    match res {
        Ok(res) => {
            trace!("Successfully fetched hitokoto: {res:?}");
            res
        }
        Err(err) => {
            warn!("Failed to fetch hitokoto: {}", err.to_string());
            Hitokoto {
                hitokoto: "网络错误".to_string(),
                from: "琳酱".to_string(),
            }
        }
    }
}

fn send_hitokoto(ctx: &mut Context, _message: &Message) -> Consumption {
    let args = ctx.cmd?.content;
    let args = args.split_whitespace().collect::<Vec<_>>().join("&c=");
    let ctx = ctx.task();

    Consumption::StopWith(Box::pin(async move {
        let res = get_hitokoto(&args).await;

        let res = ctx
            .reply(format!("{} ——{}", res.hitokoto, res.from))
            .send()
            .await;

        if let Err(err) = res {
            warn!("Failed to send reply: {}", err.to_string());
        }
    }))
}

pub static MODULE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "hitokoto",
        description: "获取一言",
        description_detailed: Some(concat!(
            "可选参数：用空格分割的 hitokoto API 类别列表\n",
            "a	动画\n",
            "b	漫画\n",
            "c	游戏\n",
            "d	文学\n",
            "e	原创\n",
            "f	来自网络\n",
            "g	其他\n",
            "h	影视\n",
            "i	诗词\n",
            "j	网易云\n",
            "k	哲学\n",
            "l	抖机灵\n",
            "其他	作为 动画 类型处理"
        )),
    }),
    task: send_hitokoto,
};
