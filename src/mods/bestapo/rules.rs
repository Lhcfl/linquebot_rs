use super::utils::{is_contains_url, is_zero_width_char};
use crate::utils::telegram::prelude::MessageExtension;
use regex::Regex;
use std::sync::LazyLock;
use teloxide_core::types::Message;
use unicode_segmentation::UnicodeSegmentation;

const SENSITIVE_WORDS: &[&str] = &["trump", "nft", "opensea"];

static GROUP_HELP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)group\s+help").unwrap());

// Match one emoji grapheme, including flags, keycaps, skin tones and ZWJ sequences.
// Exclude bare digits, regional indicators and modifiers from the base character.
static EMOJI: LazyLock<Regex> = LazyLock::new(|| {
    let base =
        r"[\p{Emoji}--[0-9#*\p{Regional_Indicator}\p{Emoji_Modifier}]]\x{FE0F}?\p{Emoji_Modifier}?";
    Regex::new(&format!(
        r"\A(?:[0-9#*]\x{{FE0F}}?\x{{20E3}}|\p{{Regional_Indicator}}{{2}}|{base}(?:\x{{200D}}{base})*|\x{{1F3F4}}[\x{{E0020}}-\x{{E007E}}]+\x{{E007F}})\z"
    ))
    .unwrap()
});

struct Rule {
    name: &'static str,
    matches: fn(&Message) -> bool,
}

const RULES: &[Rule] = &[
    Rule {
        name: "sensitive channel reply",
        matches: sensitive_channel_reply,
    },
    Rule {
        name: "emoji bot with inline buttons",
        matches: emoji_bot_with_buttons,
    },
    Rule {
        name: "Group Help sender",
        matches: group_help_sender,
    },
];

pub(super) fn matching_rule(msg: &Message) -> Option<&'static str> {
    RULES
        .iter()
        .find(|rule| (rule.matches)(msg))
        .map(|rule| rule.name)
}

fn sensitive_channel_reply(msg: &Message) -> bool {
    msg.is_reply_to_channel()
        && is_contains_url(msg)
        && msg.text().is_some_and(|text| {
            let text = text.to_lowercase().replace(is_zero_width_char, "");
            SENSITIVE_WORDS.iter().any(|word| text.contains(word))
        })
}

fn emoji_bot_with_buttons(msg: &Message) -> bool {
    msg.from.as_ref().is_some_and(|user| user.is_bot)
        && msg
            .reply_markup()
            .is_some_and(|markup| markup.inline_keyboard.iter().map(Vec::len).sum::<usize>() > 2)
        && msg.text().is_some_and(|text| {
            let text = text.trim();
            !text.is_empty()
                && text.graphemes(true).all(|grapheme| {
                    grapheme.chars().all(char::is_whitespace) || EMOJI.is_match(grapheme)
                })
        })
}

fn group_help_sender(msg: &Message) -> bool {
    // Telegram handles cannot contain spaces; match the sender's display name.
    msg.from
        .as_ref()
        .is_some_and(|user| user.is_bot && GROUP_HELP.is_match(&user.full_name()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn message(text: &str, is_bot: bool, rows: &[usize]) -> Value {
        json!({
            "message_id": 1,
            "date": 0,
            "chat": {"id": -1, "type": "supergroup", "title": "test"},
            "from": {"id": 2, "is_bot": is_bot, "first_name": "sender"},
            "text": text,
            "reply_markup": {"inline_keyboard": rows.iter().map(|count| {
                (0..*count).map(|_| json!({"text": "button", "callback_data": "test"})).collect::<Vec<_>>()
            }).collect::<Vec<_>>()}
        })
    }

    fn matches(value: Value) -> bool {
        matching_rule(&serde_json::from_value(value).unwrap()).is_some()
    }

    #[test]
    fn emoji_bots_match_without_channel_reply_or_link() {
        for text in [
            "😀",
            " 👨‍👩‍👧‍👦\n👍🏽 ❤️ 🇨🇳 1️⃣ #️⃣ *️⃣ ",
            "❤",
            "🏳️‍🌈",
            "🏴\u{e0067}\u{e0062}\u{e007f}",
        ] {
            assert!(matches(message(text, true, &[3])), "{text:?}");
            assert!(matches(message(text, true, &[1, 1, 1])), "{text:?}");
        }
    }

    #[test]
    fn emoji_rule_requires_bot_emoji_body_and_more_than_two_buttons() {
        for text in [
            "",
            " \n",
            "123",
            "#*",
            "😀 hello",
            "😀广告",
            "https://example.com",
            "\u{200d}",
            "\u{fe0f}",
            "🇨",
        ] {
            assert!(!matches(message(text, true, &[3])), "{text:?}");
        }
        assert!(!matches(message("😀", false, &[3])));
        for rows in [&[][..], &[2], &[1, 1]] {
            assert!(!matches(message("😀", true, rows)));
        }
        for field in ["from", "text", "reply_markup"] {
            let mut msg = message("😀", true, &[3]);
            msg.as_object_mut().unwrap().remove(field);
            assert!(!matches(msg), "missing {field}");
        }
    }

    #[test]
    fn group_help_requires_bot_but_not_channel_reply_or_link() {
        for name in ["Group Help", "prefix GROUP   help suffix"] {
            let mut msg = message("ordinary text", true, &[]);
            msg["from"]["first_name"] = json!(name);
            assert!(matches(msg.clone()));
            msg["from"]["is_bot"] = json!(false);
            assert!(!matches(msg));
        }
        let mut msg = message("ordinary text", true, &[]);
        msg["from"]["first_name"] = json!("Group");
        msg["from"]["last_name"] = json!("Help");
        msg.as_object_mut().unwrap().remove("text");
        assert!(matches(msg.clone()));
        msg["from"]["is_bot"] = json!(false);
        assert!(!matches(msg));
        for name in ["Group", "Help", "Group Helper"] {
            let mut msg = message("Group Help", true, &[]);
            msg["from"]["first_name"] = json!(name);
            // The requested substring match also includes "Group Helper".
            assert_eq!(matches(msg), name == "Group Helper");
        }
    }

    #[test]
    fn sensitive_words_still_require_channel_reply_and_link() {
        let mut msg = message("N\u{200b}FT", false, &[]);
        msg["link_preview_options"] = json!({"url": "https://example.com"});
        assert!(!matches(msg.clone()));
        msg["reply_to_message"] = json!({
            "message_id": 2, "date": 0, "chat": msg["chat"],
            "sender_chat": {"id": -2, "type": "channel", "title": "channel"},
            "text": "channel post"
        });
        assert!(matches(msg.clone()));
        msg["text"] = json!("ordinary text");
        assert!(!matches(msg.clone()));
        msg["text"] = json!("trump opensea");
        assert!(matches(msg.clone()));
        msg["link_preview_options"]["is_disabled"] = json!(true);
        assert!(!matches(msg.clone()));
        msg.as_object_mut().unwrap().remove("link_preview_options");
        assert!(!matches(msg));
    }
}
