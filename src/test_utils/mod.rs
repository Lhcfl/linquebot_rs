use chrono::{DateTime, Utc};
use teloxide_core::types::*;

pub fn fake_chat() -> Chat {
    Chat {
        id: ChatId(5678),
        kind: ChatKind::Public(ChatPublic {
            title: Some("fake chat".to_string()),
            kind: PublicChatKind::Group(PublicChatGroup { permissions: None }),
            description: Some("fake chat description".to_string()),
            invite_link: None,
            has_protected_content: None,
        }),
        photo: None,
        available_reactions: None,
        pinned_message: None,
        message_auto_delete_time: None,
        has_hidden_members: false,
        has_aggressive_anti_spam_enabled: false,
        chat_full_info: ChatFullInfo {
            accent_color_id: None,
            background_custom_emoji_id: None,
            profile_accent_color_id: None,
            profile_background_custom_emoji_id: None,
            emoji_status_custom_emoji_id: None,
            emoji_status_expiration_date: None,
            has_visible_history: false,
        },
    }
}

pub fn fake_text_message(text: &str) -> Message {
    Message {
        id: MessageId(1234),
        thread_id: None,
        from: None,
        sender_chat: None,
        date: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        chat: fake_chat(),
        is_topic_message: false,
        via_bot: None,
        kind: MessageKind::Common(MessageCommon {
            author_signature: None,
            forward_origin: None,
            reply_to_message: None,
            external_reply: None,
            quote: None,
            edit_date: None,
            media_kind: MediaKind::Text(MediaText {
                text: text.to_string(),
                entities: Vec::new(),
                link_preview_options: None,
            }),
            reply_markup: None,
            is_automatic_forward: false,
            has_protected_content: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;

    #[test]
    fn fake_message_tests() {
        assert_eq!(fake_text_message("hello").text(), Some("hello"));
    }
}
