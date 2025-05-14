use teloxide_core::types::Message;

pub fn is_contains_url(msg: &Message) -> bool {
    msg.link_preview_options()
        .is_some_and(|opt| opt.url.is_some() && !opt.is_disabled)
}

pub fn is_zero_width_char(chr: char) -> bool {
    matches!(chr, '\u{2062}' | '\u{200B}' | '\u{200C}' | '\u{200D}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_contains_url() {
        let msg_with_zero_width = "⁢⁢y⁢⁢o⁢⁢ ⁢⁢g⁢⁢e⁢⁢t⁢⁢ ⁢⁢f⁢⁢r⁢⁢e⁢⁢e⁢⁢ ⁢⁢N⁢⁢F⁢⁢T⁢⁢  https://faly.world/";
        let msg_trimed = msg_with_zero_width.replace(is_zero_width_char, "");
        assert_eq!(msg_trimed, "yo get free NFT  https://faly.world/");
    }
}
