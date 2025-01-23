use std::sync::Arc;

use types::ConsumeKind;

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
        Consume(Option<Pin<Box<dyn Future<Output = ()> + Send>>>),
    }

    impl Debug for ConsumeKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Decline => write!(f, "Decline"),
                Self::Consume(act) => f
                    .debug_tuple("Consume")
                    .field(if act.is_some() { &"Some" } else { &"None" })
                    .finish(),
            }
        }
    }

    impl<T: Future<Output = ()> + Send + 'static> From<T> for ConsumeKind {
        fn from(fut: T) -> Self {
            Self::Consume(Some(Box::pin(fut)))
        }
    }

    impl Try for ConsumeKind {
        type Output = Option<Pin<Box<dyn Future<Output = ()> + Send>>>;
        type Residual = Self;
        fn from_output(opt: Self::Output) -> Self {
            Self::Consume(opt)
        }

        fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
            match self {
                ConsumeKind::Decline => std::ops::ControlFlow::Break(self),
                ConsumeKind::Consume(fut) => std::ops::ControlFlow::Continue(fut),
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

mod msg_context;
pub use msg_context::*;

pub struct CommandInfo {
    name: &'static str,
    description: &'static str,
    handler: fn(&MsgContext) -> ConsumeKind,
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
    fn match_message(&self, ctx: Arc<MsgContext>) -> ConsumeKind;
}

pub trait BotRegistryExt {
    fn dispatch_command(&self, ctx: &MsgContext) -> ConsumeKind;
}
impl<T: BotRegistry> BotRegistryExt for T {
    fn dispatch_command(&self, ctx: &MsgContext) -> ConsumeKind {
        for info in self.commands() {
            if let res @ ConsumeKind::Consume(_) = (info.handler)(ctx) {
                return res;
            }
        }
        ConsumeKind::Decline
    }
}
