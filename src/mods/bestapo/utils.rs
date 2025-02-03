use teloxide_core::types::MediaKind::Text;
use teloxide_core::types::{ChatKind, Message, MessageKind::Common, PublicChatKind::Channel};

pub fn is_channel_msg_reply(msg: &Message) -> bool {
    match &msg.kind {
        Common(comm) => {
            let kind = comm
                .reply_to_message
                .as_ref()
                .and_then(|msg| msg.sender_chat.as_ref())
                .and_then(|chat| Some(&chat.kind));
            return matches!(kind, Some(ChatKind::Public(chat)) if matches!(chat.kind, Channel(_)));
        }
        _ => return false,
    };
}

pub fn is_contains_url(msg: &Message) -> bool {
    match &msg.kind {
        Common(msgc) => match &msgc.media_kind {
            Text(text) => {
                let url = text
                    .link_preview_options
                    .as_ref()
                    .and_then(|link_preview_options| link_preview_options.url.as_ref());
                let preview_disabled = text
                    .link_preview_options
                    .as_ref()
                    .is_some_and(|link_preview_options| link_preview_options.is_disabled);
                if url.is_some() && !preview_disabled {
                    return true;
                }
                return false;
            }
            _ => return false,
        },
        _ => return false,
    }
}
