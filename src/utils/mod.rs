pub fn escape_html(str: &str) -> String {
    let mut ret = String::new();
    ret.reserve(str.len() * 2);
    for ch in str.chars() {
        match ch {
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            '&' => ret.push_str("&amp"),
            x => ret.push(x),
        }
    }
    ret
}

pub fn has_command(str: &str, cmd: &str) -> bool {
    str == format!("/{cmd}") || str.starts_with(&format!("/{cmd} "))
}

pub mod telegram {
    pub mod prelude {
        use teloxide_core::types::User;

        pub trait UserExtension {
            fn html_link(&self) -> String;
        }

        impl UserExtension for User {
            fn html_link(&self) -> String {
                format!("<a href=\"{}\">{}</a>", self.url(), self.full_name())
            }
        }
    }
}
