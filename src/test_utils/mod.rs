#[cfg(test)]
pub mod fabricator {
    use chrono::{DateTime, Utc};
    use teloxide_core::types::*;

    pub fn fab_chat() -> Chat {
        Chat {
            id: ChatId(5678),
            kind: ChatKind::Public(ChatPublic {
                title: Some("fake chat".to_string()),
                kind: PublicChatKind::Group,
            }),
        }
    }

    pub fn fab_text_message(text: &str) -> Message {
        Message {
            id: MessageId(1234),
            thread_id: None,
            from: None,
            sender_chat: None,
            date: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            chat: fab_chat(),
            is_topic_message: false,
            via_bot: None,
            sender_business_bot: None,
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
                paid_star_count: None,
                business_connection_id: None,
                effect_id: None,
                is_from_offline: false,
                reply_to_story: None,
                sender_boost_count: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::fabricator::*;

    #[test]
    fn fake_message_tests() {
        assert_eq!(fab_text_message("hello").text(), Some("hello"));
    }
}
