pub mod db;
pub mod msg_context;

use std::{future::Future, pin::Pin};

use msg_context::{CmdParts, Context};
use teloxide_core::{
    prelude::*,
    types::{CallbackQuery, ChatMemberUpdated, Message},
};

use crate::DataStorage;

pub type TaskResult = Pin<Box<dyn Future<Output = ()> + Send>>;

pub mod types {
    use std::{
        convert::Infallible,
        fmt::Debug,
        future::Future,
        ops::{FromResidual, Try},
    };

    use super::TaskResult;

    // pub enum Consumption {
    //     Next,
    //     Stop,
    //     StopWith(super::TaskResult),
    // }

    pub struct Consumption {
        pub next: bool,
        pub tasks: Vec<TaskResult>,
    }

    impl Consumption {
        pub fn just_next() -> Self {
            Self {
                tasks: Vec::new(),
                next: true,
            }
        }
        pub fn just_stop() -> Self {
            Self {
                tasks: Vec::new(),
                next: false,
            }
        }
    }

    impl Debug for Consumption {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Consumption[Next = {}, tasks = [TaskResult; {}]]",
                self.next,
                self.tasks.len()
            )
        }
    }

    impl PartialEq for Consumption {
        fn eq(&self, other: &Self) -> bool {
            self.next == other.next && self.tasks.len() == other.tasks.len()
        }
    }

    impl<T: Future<Output = ()> + Send + 'static> From<T> for Consumption {
        fn from(fut: T) -> Self {
            Self {
                next: false,
                tasks: vec![Box::pin(fut)],
            }
        }
    }

    // impl Try for Consumption {
    //     type Output = Option<TaskResult>;
    //     type Residual = Self;
    //     fn from_output(opt: Self::Output) -> Self {
    //         match opt {
    //             Some(fut) => fut.into(),
    //             None => Consumption::Stop,
    //         }
    //     }

    //     fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
    //         match self {
    //             Consumption::just_next() => std::ops::ControlFlow::Break(self),
    //             Consumption::Stop => std::ops::ControlFlow::Continue(None),
    //             Consumption::StopWith(fut) => std::ops::ControlFlow::Continue(Some(fut)),
    //         }
    //     }
    // }

    // impl FromResidual for Consumption {
    //     fn from_residual(res: <Self as Try>::Residual) -> Self {
    //         res
    //     }
    // }

    impl FromResidual<Option<Infallible>> for Consumption {
        fn from_residual(None: Option<Infallible>) -> Self {
            Self::just_next()
        }
    }
    impl FromResidual<Result<Infallible, ()>> for Consumption {
        fn from_residual(Err(()): Result<Infallible, ()>) -> Self {
            Self::just_next()
        }
    }
}

/// 一个模块的描述
#[derive(Debug)]
pub struct ModuleDescription {
    /// 对于 [ModuleKind::Command] name 即命令的名字，会被预先匹配
    pub name: &'static str,
    /// 在 help 和 my_commands 中显示的提示信息
    pub description: &'static str,
    /// 单独显示的详细提示。当为 None 时，help 页不会显示详细信息按钮。
    pub description_detailed: Option<&'static str>,
}

/// 模块类型
pub enum ModuleKind {
    /// `Command` 只响应 `/command` 形状的消息。命令名是预处理的，只会被解析一次。
    Command(ModuleDescription),
    /// `General` 对模块响应的消息形状不做要求，自行处理  
    /// 当一个 `General` 模块没有描述的时候，它不会显示在 `/help` 中
    General(Option<ModuleDescription>),
}

/// 其他 Telegram Updates 的响应器
pub enum MicroTask {
    OnCallbackQuery(fn(app: &'static App, query: &CallbackQuery) -> types::Consumption),
    OnMyChatMember(fn(app: &'static App, data: &ChatMemberUpdated) -> types::Consumption),
}

/// 消息处理模块
pub struct Module {
    pub kind: ModuleKind,
    pub task: fn(ctx: &mut Context, message: &Message) -> types::Consumption,
}

impl Module {
    pub fn name(&self) -> Option<&'static str> {
        match &self.kind {
            ModuleKind::Command(c) => Some(c.name),
            ModuleKind::General(Some(g)) => Some(g.name),
            _ => None,
        }
    }
}

/// 在整个琳酱初始化后一直存在的 App 实例  
/// 存放预先 fetch 的 bot 数据和模块列表等
pub struct App {
    /// telegram bot id
    pub bot_id: UserId,
    /// telegram bot username
    pub username: String,
    /// teloxide bot instance
    pub bot: Bot,
    /// database
    pub db: DataStorage,
    /// modules loaded
    pub modules: &'static [&'static Module],
    /// micor_tasks loaded
    pub micro_tasks: &'static [&'static MicroTask],
}

impl App {
    pub fn create_message_context<'a>(&'static self, message: &'a Message) -> Context<'a> {
        Context {
            cmd: CmdParts::parse_from(message),
            message_id: message.id,
            chat_id: message.chat.id,
            app: self,
        }
    }
}
