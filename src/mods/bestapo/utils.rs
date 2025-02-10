use teloxide_core::types::Message;

pub fn is_contains_url(msg: &Message) -> bool {
    msg.link_preview_options()
        .is_some_and(|opt| opt.url.is_some() && !opt.is_disabled)
}
