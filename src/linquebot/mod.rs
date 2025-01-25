pub mod msg_context;

use msg_context::{CmdParts, Context};
use std::{future::Future, pin::Pin};
use teloxide_core::{prelude::*, types::Message};

use crate::db::DataStorage;
type TaskResult = Pin<Box<dyn Future<Output = ()> + Send>>;

pub mod types {
    use std::{
        convert::Infallible,
        fmt::Debug,
        future::Future,
        ops::{FromResidual, Try},
        pin::Pin,
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

pub struct ModuleDesctiption {
    pub name: &'static str,
    pub description: &'static str,
    pub description_detailed: Option<&'static str>,
}

pub enum ModuleKind {
    Command(ModuleDesctiption),
    General(Option<ModuleDesctiption>),
}

pub struct Module {
    pub kind: ModuleKind,
    pub task: fn(ctx: &mut Context, message: &Message) -> types::Consumption,
}

pub struct App {
    pub name: String,
    pub username: String,
    pub bot: Bot,
    pub db: DataStorage,
    pub modules: &'static [&'static Module],
}

impl App {
    pub fn parse_command<'a>(&self, str: &'a str, cmd: &str) -> Option<&'a str> {
        if str == format!("/{cmd}") {
            return Some("");
        }
        let bot_username = &self.username;
        if str == format!("/{cmd}@{bot_username}") {
            return Some("");
        }
        let t = format!("/{cmd} ");
        if str.starts_with(&t) {
            return Some(&str[t.len()..].trim());
        }
        let t = format!("/{cmd}@{bot_username} ");
        if str.starts_with(&t) {
            return Some(&str[t.len()..].trim());
        }
        return None;
    }

    pub fn create_message_context<'a>(&'static self, message: &'a Message) -> Context<'a> {
        Context {
            cmd: CmdParts::parse_from(message),
            message_id: message.id,
            chat_id: message.chat.id,
            app: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use teloxide_core::Bot;

    use crate::{db::DataStorage, App};

    fn fake_app() -> App {
        App {
            name: "琳酱".to_string(),
            username: "testbot".to_string(),
            bot: Bot::new("fake:token"),
            db: DataStorage {},
            modules: &[],
        }
    }

    #[test]
    fn parse_command_tests() {
        let app = fake_app();

        assert_eq!(app.parse_command("你好", "some"), None);
        assert_eq!(app.parse_command("some", "some"), None);
        assert_eq!(app.parse_command(" /some test 123", "some"), None);

        assert_eq!(app.parse_command("/some", "some"), Some(""));
        assert_eq!(app.parse_command("/some ", "some"), Some(""));
        assert_eq!(app.parse_command("/some   ", "some"), Some(""));
        assert_eq!(app.parse_command("/some   123", "some"), Some("123"));
        assert_eq!(
            app.parse_command("/some test 123  ", "some"),
            Some("test 123")
        );
        assert_eq!(
            app.parse_command("/some test  123", "some"),
            Some("test  123")
        );

        assert_eq!(app.parse_command("你好@testbot", "some"), None);
        assert_eq!(app.parse_command("some@testbot", "some"), None);
        assert_eq!(app.parse_command(" /some test 123", "some"), None);
        assert_eq!(app.parse_command("/some@otherbot", "some"), None);
        assert_eq!(app.parse_command("/some@otherbot 1 2 3", "some"), None);

        assert_eq!(app.parse_command("/some@testbot", "some"), Some(""));
        assert_eq!(app.parse_command("/some@testbot ", "some"), Some(""));
        assert_eq!(app.parse_command("/some@testbot   ", "some"), Some(""));
        assert_eq!(
            app.parse_command("/some@testbot   123", "some"),
            Some("123")
        );
        assert_eq!(
            app.parse_command("/some@testbot test 123  ", "some"),
            Some("test 123")
        );
        assert_eq!(
            app.parse_command("/some@testbot test  123", "some"),
            Some("test  123")
        );
    }
}
