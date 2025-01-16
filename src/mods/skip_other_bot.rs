//! skip other bot commands
use crate::globals::BOT_USERNAME;
use crate::ComsumedType;
use log::debug;
use regex::Regex;
use teloxide_core::prelude::*;
use teloxide_core::types::*;

pub fn on_message(_bot: &Bot, message: &Message) -> Option<ComsumedType> {
    let re = Regex::new(r"^/[\S]+?@(\w+)").unwrap();
    let text = message.text()?;
    let bot_name = BOT_USERNAME.get().expect("should has bot username");
    if let Some(caps) = re.captures(text) {
        if &caps[1] == bot_name {
            None
        } else {
            debug!(
                "message skiped because bot name ({}) != message @ bot name ({})",
                bot_name, &caps[1]
            );
            Some(ComsumedType::Stop)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use regex::Regex;
    use teloxide_core::Bot;

    #[test]
    fn skip_message_tests() {
        use crate::globals::BOT_USERNAME;
        use crate::mods::skip_other_bot::on_message;
        use crate::ComsumedType;

        BOT_USERNAME
            .set(String::from("testbot"))
            .expect("should able to set");

        let bot = Bot::new("AAAAA:ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");

        let re = Regex::new(r"^/[\S]+?@(\w+)").unwrap();
        let res = re.captures("/hello@otherbot 123").unwrap();
        assert_eq!(&res[1], "otherbot");

        assert_eq!(on_message(&bot, &fake_text_message("hello")), None);
        assert_eq!(on_message(&bot, &fake_text_message("/hello")), None);
        assert_eq!(on_message(&bot, &fake_text_message(" /hello")), None);
        assert_eq!(
            on_message(&bot, &fake_text_message("/hello@testbot 321")),
            None
        );
        assert_eq!(
            on_message(&bot, &fake_text_message("/hello@otherbot 123")),
            Some(ComsumedType::Stop)
        );
    }
}
