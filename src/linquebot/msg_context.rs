use teloxide_core::types::Message;

pub struct CmdParts<'a> {
    cmd: &'a str,
    username: Option<&'a str>,
    content: &'a str,
}

impl<'a> CmdParts<'a> {
    pub fn parse_from(msg: &'a Message) -> Option<CmdParts<'a>> {
        let text = msg.text()?;
        let mut chars = text.chars();
        let Some('/') = chars.next() else {
            return None;
        };
        let mut cmd_idx: usize = 0;
        let mut idx: usize = 1;
        for ch in chars {
            if ch == '@' {
                cmd_idx = idx;
            }
            if ch.is_whitespace() {
                break;
            }
            idx += 1;
        }
        if cmd_idx == 0 {
            Some(CmdParts {
                cmd: &text[1..idx],
                username: None,
                content: &text[idx..],
            })
        } else {
            Some(CmdParts {
                cmd: &text[1..cmd_idx],
                username: Some(&text[cmd_idx..idx]),
                content: &text[idx..],
            })
        }
    }
}

pub struct MsgContext<'a> {
    cmd: Option<CmdParts<'a>>,
}

impl<'a> MsgContext<'a> {
    pub fn new(msg: &'a Message) -> MsgContext<'a> {
        return MsgContext {
            cmd: CmdParts::parse_from(msg),
        };
    }
}
