pub mod db;
pub mod msg_context;

use std::{future::Future, pin::Pin};

use msg_context::{CmdParts, Context};
use teloxide_core::{
    prelude::*,
    types::{CallbackQuery, ChatMemberUpdated, Message},
};

use crate::DataStorage;

type TaskResult = Pin<Box<dyn Future<Output = ()> + Send>>;

pub mod types {
    use std::{
        convert::Infallible,
        fmt::Debug,
        future::Future,
        ops::{FromResidual, Try},
    };

    use super::TaskResult;

    pub enum Consumption {
        Next,
        Stop,
        StopWith(super::TaskResult),
    }

    impl Debug for Consumption {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Next => write!(f, "Next"),
                Self::Stop => write!(f, "Stop"),
                Self::StopWith(_) => write!(f, "StopWith(...)"),
            }
        }
    }

    impl PartialEq for Consumption {
        fn eq(&self, other: &Self) -> bool {
            if let Self::Next = self {
                if let Self::Next = other {
                    return true;
                }
            }
            if let Self::Stop = self {
                if let Self::Next = other {
                    return true;
                }
            }
            if let Self::StopWith(_) = self {
                if let Self::StopWith(_) = other {
                    return true;
                }
            }
            false
        }
    }

    impl<T: Future<Output = ()> + Send + 'static> From<T> for Consumption {
        fn from(fut: T) -> Self {
            Self::StopWith(Box::pin(fut))
        }
    }

    impl Try for Consumption {
        type Output = Option<TaskResult>;
        type Residual = Self;
        fn from_output(opt: Self::Output) -> Self {
            match opt {
                Some(fut) => fut.into(),
                None => Consumption::Stop,
            }
        }

        fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
            match self {
                Consumption::Next => std::ops::ControlFlow::Break(self),
                Consumption::Stop => std::ops::ControlFlow::Continue(None),
                Consumption::StopWith(fut) => std::ops::ControlFlow::Continue(Some(fut)),
            }
        }
    }

    impl FromResidual for Consumption {
        fn from_residual(res: <Self as Try>::Residual) -> Self {
            res
        }
    }
    impl FromResidual<Option<Infallible>> for Consumption {
        fn from_residual(None: Option<Infallible>) -> Self {
            Self::Next
        }
    }
    impl FromResidual<Result<Infallible, ()>> for Consumption {
        fn from_residual(Err(()): Result<Infallible, ()>) -> Self {
            Self::Next
        }
    }
}

#[derive(Debug)]
pub struct ModuleDesctiption {
    pub name: &'static str,
    pub description: &'static str,
    pub description_detailed: Option<&'static str>,
}

pub enum ModuleKind {
    Command(ModuleDesctiption),
    General(Option<ModuleDesctiption>),
}

pub enum MicroTask {
    OnCallbackQuery(fn(app: &'static App, query: &CallbackQuery) -> types::Consumption),
    OnMyChatMember(fn(app: &'static App, data: &ChatMemberUpdated) -> types::Consumption),
}

pub struct Module {
    pub kind: ModuleKind,
    pub task: fn(ctx: &mut Context, message: &Message) -> types::Consumption,
}

pub struct App {
    pub bot_id: UserId,
    pub username: String,
    pub bot: Bot,
    pub db: DataStorage,
    pub modules: &'static [&'static Module],
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
