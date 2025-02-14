//! 随机群老婆
use graphviz_rust::dot_structures::Graph;
use graphviz_rust::printer::DotPrinter;
use graphviz_rust::printer::PrinterContext;
use log::error;
use log::info;
use log::warn;
use msg_context::Context;
use rand::seq::IteratorRandom;
use rand::thread_rng;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::DerefMut;
use std::time::SystemTime;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

use crate::linquebot::msg_context::TaskContext;
use crate::linquebot::*;
use crate::utils::telegram::prelude::*;
use crate::Consumption;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaifeUser {
    id: UserId,
    full_name: String,
}

impl WaifeUser {
    fn from_user(user: &User) -> Self {
        Self {
            id: user.id,
            full_name: user.full_name(),
        }
    }

    fn html_link(&self) -> String {
        format!("<b>{}</b>", self.full_name)
    }

    fn escaped_name(&self) -> String {
        let mut res = String::new();
        for ch in self.full_name.chars() {
            match ch {
                '\\' => res.push_str("\\\\"),
                '"' => res.push_str("\\\""),
                ch => res.push(ch),
            }
        }
        res
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WaifeStatus {
    users: HashMap<UserId, WaifeUser>,
    last_waife_date: SystemTime,
    waife_of: HashMap<UserId, HashSet<UserId>>, // set 里面的用户 id 全都是前者的老婆！多元关系！
}

#[derive(Debug, Serialize, Deserialize)]
struct UserChatCache {
    cache_at: SystemTime,
    joined: bool,
}

impl UserChatCache {
    fn new(joined: bool) -> Self {
        Self {
            cache_at: SystemTime::now(),
            joined,
        }
    }

    fn invalid(&self) -> bool {
        SystemTime::now()
            .duration_since(self.cache_at)
            .is_ok_and(|d| d.as_secs() > 3600)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserCache {
    chats: HashMap<ChatId, UserChatCache>,
    // avatar: Option<...>
}

fn get_waife(ctx: &mut Context, msg: &Message) -> Consumption {
    let from = WaifeUser::from_user(msg.from.as_ref()?);
    let poly = ctx.cmd?.content == "poly";
    let ctx = ctx.task();
    async move {
        let mut waife_storage = match ctx.app.db.of::<WaifeStatus>().chat(ctx.chat_id).get().await {
            Some(x) => x,
            None => {
                ctx.reply("稍等 >_<").send().warn_on_error("waife").await;
                ctx.app
                    .db
                    .of::<WaifeStatus>()
                    .chat(ctx.chat_id)
                    .get()
                    .await
                    .unwrap()
            }
        };

        let now = SystemTime::now();
        let Ok(duration) = now.duration_since(waife_storage.last_waife_date) else {
            error!(
                "past is {:?} but now is {:?}, failed to get duration",
                waife_storage.last_waife_date, now
            );
            waife_storage.last_waife_date = now;
            ctx.reply("发生了不应该出现的时间错误，琳酱不知道哦 >_<")
                .send()
                .warn_on_error("waife")
                .await;
            return;
        };

        if duration.as_secs() > 86400 {
            waife_storage.last_waife_date = now;
            waife_storage.waife_of = HashMap::new();
            waife_storage.users.clear();
            add_admins_to_users(&mut waife_storage.users, &ctx)
                .warn_on_error("waife-auto-add")
                .await;
        }

        if !check_and_add(&mut waife_storage.users, &ctx, from.clone()).await {
            ctx.reply("失败：琳酱无法获取你在群内的状态 >_<")
                .send()
                .warn_on_error("waife")
                .await;
            return;
        }

        let WaifeStatus {
            waife_of,
            users,
            last_waife_date: _,
        } = &mut waife_storage.deref_mut();

        let waife_uids = waife_of.entry(from.id).or_default();

        // 一元关系
        if !(waife_uids.is_empty() || poly) {
            let waifes = waife_uids
                .iter()
                .map(|uid| users.get(uid).unwrap().html_link())
                .collect::<Vec<_>>();
            ctx.reply_html(format!(
                "你今天已经有老婆了，你的群老婆：{}",
                waifes.join("、"),
            ))
            .send()
            .warn_on_error("waife")
            .await;
            return;
        }

        if poly && waife_uids.len() > 1 && waife_uids.len() >= users.len() - 1 {
            ctx.reply("别贪心了，琳酱认识的群成员已经全是你老婆了！")
                .send()
                .warn_on_error("waife")
                .await;
            return;
        }

        let Some((waife_id, waife_user_html)) = users
            .iter()
            .filter(|(uid, _)| !waife_uids.contains(uid) && **uid != from.id)
            .choose(&mut thread_rng())
            .map(|(x, y)| (*x, y.html_link()))
        else {
            ctx.reply("琳酱还不认识足够多的群成员，无法为您分配随机老婆 >_<")
                .send()
                .warn_on_error("waife")
                .await;
            return;
        };

        waife_uids.insert(waife_id);

        ctx.reply_html(format!("获取成功！你今天的群老婆是 {waife_user_html}"))
            .send()
            .warn_on_error("waife")
            .await;
    }
    .into()
}

async fn add_admins_to_users(
    users: &mut HashMap<UserId, WaifeUser>,
    ctx: &TaskContext,
) -> anyhow::Result<()> {
    let admins = ctx
        .app
        .bot
        .get_chat_administrators(ctx.chat_id)
        .send()
        .await?;
    for member in admins {
        if member.user.id == ctx.app.bot_id {
            continue;
        }
        users.insert(member.user.id, WaifeUser::from_user(&member.user));
    }
    Ok(())
}

async fn check_and_add(
    users: &mut HashMap<UserId, WaifeUser>,
    ctx: &TaskContext,
    user: WaifeUser,
) -> bool {
    let mut user_cache = ctx
        .app
        .db
        .of()
        .user(user.id)
        .get_or_insert(|| UserCache {
            chats: HashMap::new(),
        })
        .await;

    if let Some(cache) = user_cache.chats.get(&ctx.chat_id) {
        if !cache.invalid() {
            // we still need update user info
            if cache.joined {
                users.insert(user.id, user);
            }
            return cache.joined;
        }
    }

    let membership = match ctx
        .app
        .bot
        .get_chat_member(ctx.chat_id, user.id)
        .send()
        .await
    {
        Ok(x) => x,
        Err(err) => {
            user_cache
                .chats
                .insert(ctx.chat_id, UserChatCache::new(false));
            warn!("failed to fetch memebership: {err}");
            return false;
        }
    };

    if membership.is_present() {
        user_cache
            .chats
            .insert(ctx.chat_id, UserChatCache::new(true));
        users.insert(user.id, user);
    } else {
        user_cache
            .chats
            .insert(ctx.chat_id, UserChatCache::new(false));
        // 即使没有在群里也不要 remove user，防止退群引起老婆图缺失。
        info!("Ignored out-of-group user：{}", user.full_name);
    }

    membership.is_present()
}

fn auto_add_user(ctx: &mut Context, msg: &Message) -> Consumption {
    if msg.chat.is_private() {
        return Consumption::Next;
    }
    // Telegram says for backward compatibility, if the message was sent on behalf of a chat,
    // the field contains a fake sender user in non-channel chats.
    // But we don't need a fake user. Drop it.
    if msg.sender_chat.is_some() {
        return Consumption::Next;
    }
    // 聊天群和绑定的 channel 可能有不同的人，为了保持 waife 不遇到晦气人，丢弃来自 forward 的消息。
    if msg.is_automatic_forward() || msg.is_reply_to_channel() {
        info!("Droped channel message/reply: {:?}", msg.text());
        return Consumption::Next;
    }
    let from = WaifeUser::from_user(msg.from.as_ref()?);
    let ctx = ctx.task();
    tokio::spawn(async move {
        let mut waife_storage = ctx
            .app
            .db
            .of::<WaifeStatus>()
            .chat(ctx.chat_id)
            .get_or_insert(|| WaifeStatus {
                users: HashMap::new(),
                last_waife_date: SystemTime::now(),
                waife_of: HashMap::new(),
            })
            .await;

        if waife_storage.users.is_empty() {
            add_admins_to_users(&mut waife_storage.users, &ctx)
                .warn_on_error("waife-auto-add")
                .await
        }
        check_and_add(&mut waife_storage.users, &ctx, from).await;
    });
    Consumption::Next
}

#[derive(Clone, Copy)]
enum WaifeGraphGenerator {
    Auto,
    Dot,
    Neato,
    Fdp,
    Circo,
}

async fn generate_waife_graph(
    app: &'static App,
    chat_id: ChatId,
    perfer: WaifeGraphGenerator,
) -> Result<Graph, &'static str> {
    let Some(waife_storage) = app.db.of::<WaifeStatus>().chat(chat_id).get().await else {
        return Err("群里还没人有老婆哦");
    };

    if waife_storage.waife_of.is_empty() {
        return Err("群里还没人有老婆哦");
    };

    use graphviz_rust::dot_generator::*;
    use graphviz_rust::dot_structures::*;

    let mut res = Graph::DiGraph {
        id: Id::Anonymous(String::new()),
        strict: false,
        stmts: Vec::new(),
    };

    let mut used_userids = HashSet::new();

    let mut edge_count = 0;
    let mut stop_using_dot = false;

    for (user_id, waife_ids) in waife_storage.waife_of.iter() {
        used_userids.insert(*user_id);
        used_userids.extend(waife_ids.iter());
        stop_using_dot = stop_using_dot || waife_ids.len() > 4;
        edge_count += waife_ids.len();

        for waife_id in waife_ids {
            res.add_stmt(Stmt::Edge(edge!(node_id!(user_id) => node_id!(waife_id))));
        }
    }

    let mut perfer = perfer;

    match perfer {
        WaifeGraphGenerator::Auto => {
            if stop_using_dot {
                if used_userids.len() >= 5
                    && used_userids.len() < 12
                    && edge_count > used_userids.len() * used_userids.len() / 4
                {
                    perfer = WaifeGraphGenerator::Circo;
                } else if used_userids.len() > 100 {
                    perfer = WaifeGraphGenerator::Fdp;
                } else {
                    perfer = WaifeGraphGenerator::Neato
                }
            } else {
                perfer = WaifeGraphGenerator::Dot;
            }
        }
        WaifeGraphGenerator::Neato => {
            if used_userids.len() > 100 {
                perfer = WaifeGraphGenerator::Fdp;
            }
        }
        _ => {}
    };

    match perfer {
        WaifeGraphGenerator::Auto => unreachable!(),
        WaifeGraphGenerator::Dot => {}
        WaifeGraphGenerator::Neato => {
            res.add_stmt(Stmt::GAttribute(GraphAttributes::Graph(vec![
                attr!("layout", "neato"),
                attr!("overlap", "false"),
            ])));
        }
        WaifeGraphGenerator::Fdp => {
            res.add_stmt(Stmt::GAttribute(GraphAttributes::Graph(vec![attr!(
                "layout", "fdp"
            )])));
        }
        WaifeGraphGenerator::Circo => {
            res.add_stmt(Stmt::GAttribute(GraphAttributes::Graph(vec![
                attr!("layout", "circo"),
                attr!("mindist", "0.2"),
            ])));
        }
    }

    for user_id in used_userids {
        let Some(user) = waife_storage.users.get(&user_id) else {
            error!(
                "need user_id: {user_id}, but storage: {:?}",
                waife_storage.users
            );
            return Err("内部错误，但这不太可能发生……");
        };
        res.add_stmt(Stmt::Node(
            // why the fuck `esc` here not work?
            node!(user_id; attr!("shape", "box"), attr!(esc "label", esc user.escaped_name())),
        ));
    }

    Ok(res)
}

fn on_waife_graph(ctx: &mut Context, _: &Message) -> Consumption {
    let perfer = match ctx.cmd?.content {
        "dot" => WaifeGraphGenerator::Dot,
        "fdp" => WaifeGraphGenerator::Fdp,
        "neato" => WaifeGraphGenerator::Neato,
        "circo" => WaifeGraphGenerator::Circo,
        _ => WaifeGraphGenerator::Auto,
    };
    let ctx = ctx.task();
    async move {
        match generate_waife_graph(ctx.app, ctx.chat_id, perfer).await {
            Ok(graph) => {
                ctx.app
                    .bot
                    .send_chat_action(ctx.chat_id, ChatAction::UploadPhoto)
                    .send()
                    .warn_on_error("waife-graph")
                    .await;

                let png = graphviz_rust::exec(
                    graph.clone(),
                    &mut PrinterContext::default(),
                    vec![graphviz_rust::cmd::Format::Png.into()],
                );

                match png {
                    Ok(res) => {
                        ctx.app
                            .bot
                            .send_photo(ctx.chat_id, InputFile::memory(res))
                            .reply_parameters(ReplyParameters::new(ctx.message_id))
                            .send()
                            .warn_on_error("waife-graph")
                            .await
                    }
                    Err(err) => {
                        log::warn!(
                            "Graphviz error occurrs. The graph is:\n{}",
                            graph.print(&mut PrinterContext::default())
                        );
                        let mut err_str = err.to_string();
                        if err_str.contains("program not found") || err_str.contains("No such file")
                        {
                            err_str = "\n看上去琳酱缺少了依赖：Graphviz，请联系琳酱部署者安装"
                                .to_string();
                        }
                        ctx.reply(format!("琳酱在生成老婆图的时候发生了意外错误…… {err_str}"))
                            .send()
                            .warn_on_error("waife-graph")
                            .await
                    }
                }
            }
            Err(msg) => ctx.reply(msg).send().warn_on_error("waife-graph").await,
        }
    }
    .into()
}

pub static ADD_USER: Module = Module {
    kind: ModuleKind::General(None),
    task: auto_add_user,
};

pub static GET_WAIFE: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "waife",
        description: "获取随机群老婆",
        description_detailed: Some(concat!(
            "从bot认识的群成员中获取随机群老婆\n",
            "琳酱会认识加入以来所有发言的用户和群管理员\n",
            "特别的，<code>/waife poly</code>可以让你有多个老婆——我们支持多元关系😋"
        )),
    }),
    task: get_waife,
};

pub static WAIFE_GRAPH: Module = Module {
    kind: ModuleKind::Command(ModuleDescription {
        name: "waife_graph",
        description: "绘制群老婆图",
        description_detailed: Some(concat!(
            "使用 Graphviz 绘制老婆关系图。\n",
            "注：你可以选择绘制方式，当前可选有: <code>dot</code>, <code>neato</code>, <code>fdp</code>, <code>circo</code>。\n",
            "默认会智能选择最合适的。",  
        )
        ),
    }),
    task: on_waife_graph,
};
