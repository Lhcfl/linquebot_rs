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
    fn with_pop(self) -> Self {
        let mut c = self.0;
        if c[0] != '\0' {
            c[0] = '\0';
        } else if c[1] != '\0' {
            c[1] = '\0';
        } else {
            c[2] = '\0';
        }
        Self(c)
    }
    fn is_empty(self) -> bool {
        self.0 == ['\0'; 3]
    }
    fn segs(self) -> impl Iterator<Item = Self> {
        let mut seg = self;
        std::iter::from_coroutine(
            #[coroutine]
            move || loop {
                yield seg;
                seg = seg.with_pop();
                if seg.is_empty() {
                    break;
                }
            },
        )
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
            let mut sel = vec![];
            let mut pw = None;
            let mut pushsel = |pre: Gram| {
                let Some(ws) = weight.get(&pre) else { return };
                let pi = sel.len();
                sel.reserve(ws.len());
                sel.extend(ws.iter().map(|(c, w)| (*c, *w as f64)));
                let cw = sel[pi..].iter().map(|i| i.1).sum::<f64>();
                let pw = if let Some(pw) = &mut pw {
                    sel[pi..].iter_mut().for_each(|i| i.1 *= *pw / cw);
                    pw
                } else {
                    pw.insert(cw)
                };
                *pw /= pw.ln_1p();
            };
            let mut pv = pre;
            loop {
                pushsel(pv);
                pv = pv.with_pop();
                if pv.is_empty() {
                    break;
                }
            }
            let cur = if sel.is_empty() {
                '\0'
            } else {
                sel.choose_weighted(&mut rng, |v| v.1).expect("rand sel").0
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
        let mut pre = Gram::default();
        let weight = &mut db.weight;
        for ch in text.chars() {
            for seg in pre.segs() {
                *weight.entry(seg).or_default().entry(ch).or_default() += 1;
            }
            pre.push(ch);
        }
        for seg in pre.segs() {
            *weight.entry(seg).or_default().entry('\0').or_default() += 1;
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
