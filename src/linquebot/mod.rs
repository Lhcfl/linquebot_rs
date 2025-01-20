use std::{future::Future, pin::Pin};

use teloxide_core::{types::Message, Bot};
use types::ConsumeKind;

use crate::utils::MsgContext;

pub mod types {
    use std::{
        convert::Infallible,
        fmt::Debug,
        future::Future,
        ops::{FromResidual, Try},
        pin::Pin,
    };

    pub enum ConsumeKind {
        Decline,
        Action(Pin<Box<dyn Future<Output = ()> + Send>>),
        Consume,
    }

    impl Debug for ConsumeKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Decline => write!(f, "Decline"),
                Self::Action(_) => f.debug_tuple("Action").finish(),
                Self::Consume => write!(f, "Consume"),
            }
        }
    }

    impl<T: Future<Output = ()> + Send + 'static> From<T> for ConsumeKind {
        fn from(fut: T) -> Self {
            Self::Action(Box::pin(fut))
        }
    }

    impl Try for ConsumeKind {
        type Output = Option<Pin<Box<dyn Future<Output = ()> + Send>>>;
        type Residual = Self;
        fn from_output(opt: Self::Output) -> Self {
            match opt {
                Some(fut) => fut.into(),
                None => Self::Consume,
            }
        }

        fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
            match self {
                ConsumeKind::Decline => std::ops::ControlFlow::Break(self),
                ConsumeKind::Action(fut) => std::ops::ControlFlow::Continue(Some(fut)),
                ConsumeKind::Consume => std::ops::ControlFlow::Continue(None),
            }
        }
    }

    impl FromResidual for ConsumeKind {
        fn from_residual(res: <Self as Try>::Residual) -> Self {
            res
        }
    }
    impl FromResidual<Option<Infallible>> for ConsumeKind {
        fn from_residual(None: Option<Infallible>) -> Self {
            Self::Decline
        }
    }
    impl FromResidual<Result<Infallible, ()>> for ConsumeKind {
        fn from_residual(Err(()): Result<Infallible, ()>) -> Self {
            Self::Decline
        }
    }
}

pub struct CommandInfo {
    name: &'static str,
    description: &'static str,
    handler: fn(&Bot, &Message, &mut MsgContext),
}

pub trait BotRegistry: Sync {
    fn is_show_in_help(&self) -> bool {
        true
    }
    /// Retrieve the commands of the registry
    fn commands(&self) -> &[CommandInfo];
    /// Short mod description in the help menu
    fn description(&self) -> String;
    /// Long help message in dedicated help page
    fn help_info(&self) -> String;
    /// Handlers for messages
    fn match_message(&self, bot: &Bot, msg: &Message, ctx: &mut MsgContext) -> ConsumeKind;
}

pub trait BotRegistryExt {
    fn dispatch_command(&self, bot: &Bot, msg: &Message, ctx: &mut MsgContext);
}
impl<T: BotRegistry> BotRegistryExt for T {
    fn dispatch_command(&self, bot: &Bot, msg: &Message, ctx: &mut MsgContext) {
        let Some(cmd) = ctx.command() else {
            return;
        };
        for info in self.commands() {
            if info.name == cmd.name {
                (info.handler)(bot, msg, ctx);
                break;
            }
        }
    }
}
