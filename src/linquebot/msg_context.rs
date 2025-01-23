use std::{cell::LazyCell, mem::MaybeUninit, sync::Arc};

use teloxide_core::types::{ChatId, Message, UserId};

use crate::{
    utils::{
        pattern::{maybe, of_pred},
        Pattern,
    },
    Bot,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataScope {
    User(UserId),
    Chat(ChatId),
    ChatUser(ChatId, UserId),
}

pub struct ContextStorage {
    pub bot: Bot,
}

impl ContextStorage {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }

    pub fn make_context<'a>(&'a self, msg: Message) -> Arc<MsgContext> {
        let res = Box::leak(Box::new(MsgContext {
            // Safety: Promised by the function
            bot: unsafe { std::mem::transmute::<&Bot, &'static Bot>(&self.bot) },
            msg,
            cmd_parts: MaybeUninit::uninit(),
        }));
        res.cmd_parts
            .write(LazyCell::new(CmdParser(res.msg.text())));
        unsafe { Arc::from_raw(res) }
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

pub struct MsgContext {
    bot: &'static Bot,
    msg: Message,
    cmd_parts: MaybeUninit<LazyCell<Option<CmdParts<'static>>, CmdParser<'static>>>,
}

impl MsgContext {
    /// Retrieve the corresponding command part of the message
    pub fn command<'a>(&'a self) -> Option<CmdParts<'a>> {
        unsafe { **self.cmd_parts.assume_init_ref() }
    }
}
