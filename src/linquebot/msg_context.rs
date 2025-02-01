use teloxide_core::{
    payloads::SendMessage,
    prelude::*,
    requests::JsonRequest,
    types::{ChatId, Message, MessageId, ParseMode, ReplyParameters},
};

use super::{App, ModuleDesctiption};

/// Command parts of /xxx@yyy zzz
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct CmdParts<'a> {
    /// command name
    pub cmd: &'a str,
    /// the username of command@username
    pub username: Option<&'a str>,
    /// the trimed remaining part of command
    pub content: &'a str,
}

impl<'a> CmdParts<'a> {
    pub fn parse_from(msg: &'a Message) -> Option<CmdParts<'a>> {
        use crate::utils::pattern::*;
        let text = msg.text()?;
        let (content, (_, cmd, username)) = (
            "/",
            of_pred(|c| !c.is_whitespace() && c != '@'),
            maybe(("@", of_pred(|c| !c.is_whitespace()))),
        )
            .check_pattern(text)?;
        if cmd.is_empty() {
            return None;
        }
        Some(CmdParts {
            cmd,
            username: username.map(|u| u.1),
            content: content.trim(),
        })
    }
}

/// 和对应消息生命周期相等的 Context  
/// 会预先 parse 好 cmd
pub struct Context<'a> {
    pub cmd: Option<CmdParts<'a>>,
    pub chat_id: ChatId,
    pub message_id: MessageId,
    pub app: &'static App,
}

impl Context<'_> {
    pub fn task(&self) -> TaskContext {
        TaskContext {
            message_id: self.message_id,
            chat_id: self.chat_id,
            app: self.app,
        }
    }

    pub fn matches_command(&self, desc: &ModuleDesctiption) -> bool {
        let Some(cmd) = &self.cmd else {
            return false;
        };
        if let Some(username) = cmd.username {
            if username != self.app.username {
                return false;
            }
        }
        cmd.cmd == desc.name
    }
}

#[derive(Clone)]
pub struct TaskContext {
    pub chat_id: ChatId,
    pub message_id: MessageId,
    pub app: &'static App,
}

impl TaskContext {
    pub fn reply(&self, text: impl Into<String>) -> JsonRequest<SendMessage> {
        self.app
            .bot
            .send_message(self.chat_id, text)
            .reply_parameters(ReplyParameters::new(self.message_id))
    }

    pub fn reply_markdown(&self, text: impl Into<String>) -> JsonRequest<SendMessage> {
        self.reply(text).parse_mode(ParseMode::MarkdownV2)
    }

    pub fn reply_html(&self, text: impl Into<String>) -> JsonRequest<SendMessage> {
        self.reply(text).parse_mode(ParseMode::Html)
    }
}

#[cfg(test)]
mod tests {
    use crate::{msg_context::CmdParts, test_utils::fabricator::fab_text_message};

    #[test]
    fn cmd_parts_new_test() {
        assert_eq!(CmdParts::parse_from(&fab_text_message("1")), None);
        assert_eq!(CmdParts::parse_from(&fab_text_message("/")), None);
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/a")),
            Some(CmdParts {
                cmd: "a",
                username: None,
                content: "",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/test")),
            Some(CmdParts {
                cmd: "test",
                username: None,
                content: "",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/test args")),
            Some(CmdParts {
                cmd: "test",
                username: None,
                content: "args",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/test  args more spaces")),
            Some(CmdParts {
                cmd: "test",
                username: None,
                content: "args more spaces",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/test@somebot  args more spaces")),
            Some(CmdParts {
                cmd: "test",
                username: Some("somebot"),
                content: "args more spaces",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/test@@strange  args more spaces")),
            Some(CmdParts {
                cmd: "test",
                username: Some("@strange"),
                content: "args more spaces",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/中文 args   ")),
            Some(CmdParts {
                cmd: "中文",
                username: None,
                content: "args",
            })
        );
        assert_eq!(
            CmdParts::parse_from(&fab_text_message("/揉@somebot")),
            Some(CmdParts {
                cmd: "揉",
                username: Some("somebot"),
                content: "",
            })
        );
    }
}
