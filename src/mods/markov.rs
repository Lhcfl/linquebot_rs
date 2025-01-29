use std::collections::HashMap;

use rand::{rngs::SmallRng, seq::SliceRandom, thread_rng, SeedableRng};
use serde::{Deserialize, Serialize};
use teloxide_core::{
    prelude::{Request, Requester},
    types::Message,
};

use crate::{
    db::DbData, msg_context::Context, utils::telegram::prelude::WarnOnError, Consumption, Module,
};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
struct Gram([char; 3]);

impl Gram {
    fn push(&mut self, v: char) {
        let c = &mut self.0;
        (c[0], c[1], c[2]) = (c[1], c[2], v);
    }
    fn is_empty(&self) -> bool {
        self.0 == ['\0'; 3]
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Markov {
    weight: HashMap<Gram, HashMap<char, u32>>,
}

impl DbData for Markov {
    fn persistent() -> bool {
        true
    }

    fn from_str(src: &str) -> Self {
        ron::from_str(src).expect("deser error")
    }

    fn to_string(&self) -> String {
        ron::to_string(self).expect("ser error")
    }
}

pub fn on_message(ctx: &mut Context<'_>, msg: &Message) -> Consumption {
    const PROMPT: &str = "琳酱说说话";
    let text = msg.text()?;
    if !text.starts_with(PROMPT) {
        None?
    }
    let text = text.split_at(PROMPT.len()).1.trim().to_string();
    let db = ctx.app.db.of::<Markov>().get_or_insert(|| Markov {
        weight: HashMap::new(),
    });
    let ctx = ctx.task();
    async move {
        let weight = &mut db.await.weight;
        let mut pre = ['\0'; 3];
        for (i, c) in text.chars().rev().take(3).enumerate() {
            pre[2 - i] = c;
        }
        let mut pre = Gram(pre);
        let mut res = "".to_string();
        let mut rng = SmallRng::from_rng(&mut thread_rng()).expect("gen rng");
        loop {
            let cur = weight.get(&pre);
            let cur = if let Some(ws) = cur {
                let ws = ws.iter().collect::<Vec<_>>();
                *ws.choose_weighted(&mut rng, |v| *v.1)
                    .expect("rand choose")
                    .0
            } else {
                '\0'
            };
            res.push(cur);
            pre.push(cur);
            if cur == '\0' {
                break;
            }
        }
        let res = if res.is_empty() {
            "琳酱不知道哦"
        } else {
            &(text + &res)
        };
        ctx.app
            .bot
            .send_message(ctx.chat_id, res)
            .send()
            .warn_on_error("markov")
            .await;
    }
    .into()
}

pub fn train_data(ctx: &mut Context<'_>, msg: &Message) -> Consumption {
    let db = ctx.app.db.of::<Markov>().get_or_insert(|| Markov {
        weight: HashMap::new(),
    });
    let text = msg.text()?.to_string();
    if text.starts_with("/") || text.starts_with("琳酱说说话") {
        return Consumption::Next;
    }
    tokio::spawn(async move {
        let mut db = db.await;
        let mut pre = Default::default();
        let weight = &mut db.weight;
        for ch in text.chars() {
            *weight.entry(pre).or_default().entry(ch).or_default() += 1;
            pre.push(ch);
        }
    });
    Consumption::Next
}

pub static TRAIN_MOD: Module = Module {
    kind: crate::ModuleKind::General(None),
    task: train_data,
};

pub static GEN_CTNT: Module = Module {
    kind: crate::ModuleKind::General(Some(crate::ModuleDesctiption {
        name: "琳酱说说话",
        description: "让琳酱说一段话或者接一段话",
        description_detailed: Some(concat!(
            "直接说琳酱说说话来让琳酱随便说话, ",
            "<code>琳酱说说话 [一句话]</code>让琳酱接话.\n\n",
            "琳酱会从所有聊天记录里训练, 不会保存具体的聊天语料."
        )),
    })),
    task: on_message,
};
