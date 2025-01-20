use std::{cell::LazyCell, sync::Arc};

use teloxide_core::types::{ChatId, Message, UserId};

use crate::utils::{
    pattern::{maybe, of_pred},
    Pattern,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataScope {
    User(UserId),
    Chat(ChatId),
    ChatUser(ChatId, UserId),
}

pub struct ContextStorage {}

impl ContextStorage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn make_context(self: Arc<Self>, msg: &Message) -> MsgContext {
        MsgContext {
            storage: self,
            cmd_parts: LazyCell::new(CmdParser(msg.text())),
        }
    }
}

struct CmdParser<'a>(Option<&'a str>);
impl<'a> FnOnce<()> for CmdParser<'a> {
    type Output = Option<CmdParts<'a>>;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.call(())
    }
}
impl FnMut<()> for CmdParser<'_> {
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.call(())
    }
}
impl Fn<()> for CmdParser<'_> {
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        let text = self.0?;
        if '/'.check_pattern(text).is_none() {
            return None;
        }
        let (ctnt, ((_, name), user)) = (
            ('/', of_pred(|c| !c.is_whitespace() && c != '@')),
            maybe(('@', of_pred(|c| !c.is_whitespace()))),
        )
            .check_pattern(text)?;
        Some(CmdParts {
            name,
            user: user.map(|u| u.1),
            ctnt: ctnt.trim(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CmdParts<'a> {
    pub name: &'a str,
    pub user: Option<&'a str>,
    pub ctnt: &'a str,
}

/// Additional data for the message
pub struct MsgContext<'a> {
    storage: Arc<ContextStorage>,
    cmd_parts: LazyCell<Option<CmdParts<'a>>, CmdParser<'a>>,
}

impl MsgContext<'_> {
    /// Retrieve the corresponding command part of the message
    pub fn command(&self) -> Option<CmdParts<'_>> {
        *self.cmd_parts
    }
}
