use crate::{
    linquebot::{msg_context::Context, types::Consumption, Module, ModuleDescription, ModuleKind},
    utils::telegram::prelude::WarnOnError,
};
use serde::{Deserialize, Serialize};
use teloxide_core::{prelude::Request, types::Message};

#[derive(Debug, Serialize, Deserialize)]
pub struct Search {
    pub search_enabled: bool,
    pub search_recording_enabled: bool,
}

impl Search {
    pub fn default() -> Self {
        Search {
            search_enabled: false,
            search_recording_enabled: false,
        }
    }
}

fn on_toggle_recording(ctx: &mut Context, _: &Message) -> Consumption {
    let ctx = ctx.task();
    async move {
        let mut stat = ctx
            .app
            .db
            .of::<Search>()
            .chat(ctx.chat_id)
            .get_or_insert(Search::default)
            .await;

        stat.search_recording_enabled = !stat.search_recording_enabled;

        if stat.search_recording_enabled {
            ctx.reply("消息记录已打开")
        } else {
            ctx.reply("消息记录已关闭")
        }
        .send()
        .warn_on_error("toggle_search_recording")
        .await;
    }
    .into()
}

fn on_toggle_search(ctx: &mut Context, _: &Message) -> Consumption {
    let ctx = ctx.task();
    async move {
        let mut stat = ctx
            .app
            .db
            .of::<Search>()
            .chat(ctx.chat_id)
            .get_or_insert(Search::default)
            .await;

        stat.search_enabled = !stat.search_enabled;

        if stat.search_enabled {
            ctx.reply("消息搜索已打开")
        } else {
            ctx.reply("消息搜索已关闭")
        }
        .send()
        .warn_on_error("toggle_search")
        .await;
    }
    .into()
}

pub static TOGGLE_SEARCH_RECORDING: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "toggle_search_recording",
        description: "打开/关闭<b>搜索</b>模块的群组消息记录功能",
        description_detailed: Some(concat!(
            "该命令不需要参数。\n",
            "打开/关闭<b>搜索</b>模块的群组消息记录功能。\n",
        )),
    }),
    task: on_toggle_recording,
};

pub static TOGGLE_SEARCH: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "toggle_search",
        description: "打开/关闭<b>搜索</b>模块的群组消息搜索功能",
        description_detailed: Some(concat!(
            "该命令不需要参数。\n",
            "打开/关闭<b>搜索</b>模块的群组消息搜索功能。\n",
        )),
    }),
    task: on_toggle_search,
};
