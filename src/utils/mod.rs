use crate::globals::BOT_USERNAME;

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

/// Parse command  
/// matchs if the str is `/cmd` or `/cmd <rest>`  
/// returns rest trimmed
pub fn parse_command<'a>(str: &'a str, cmd: &str) -> Option<&'a str> {
    if str == format!("/{cmd}") {
        return Some("");
    }
    let bot_username = BOT_USERNAME.get().expect("should has bot username");
    if str == format!("/{cmd}@{bot_username}") {
        return Some("");
    }
    let t = format!("/{cmd} ");
    if str.starts_with(&t) {
        return Some(&str[t.len()..].trim());
    }
    let t = format!("/{cmd}@{bot_username} ");
    if str.starts_with(&t) {
        return Some(&str[t.len()..].trim());
    }
    return None;
}

pub fn split_n<const N: usize>(src: &str) -> (Vec<&str>, Option<&str>) {
    let mut it = src.split(|c: char| c.is_whitespace());
    let pre = (1..N)
        .map_while(|_| it.find(|s| !s.is_empty()))
        .collect::<Vec<_>>();
    (pre, it.remainder().and_then(|str| Some(str.trim())))
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

#[cfg(test)]
mod tests {
    #[test]
    fn parse_command_tests() {
        use crate::globals::BOT_USERNAME;
        use crate::utils::parse_command;
        BOT_USERNAME
            .set(String::from("testbot"))
            .expect("should able to set");

        assert_eq!(parse_command("你好", "some"), None);
        assert_eq!(parse_command("some", "some"), None);
        assert_eq!(parse_command(" /some test 123", "some"), None);

        assert_eq!(parse_command("/some", "some"), Some(""));
        assert_eq!(parse_command("/some ", "some"), Some(""));
        assert_eq!(parse_command("/some   ", "some"), Some(""));
        assert_eq!(parse_command("/some   123", "some"), Some("123"));
        assert_eq!(parse_command("/some test 123  ", "some"), Some("test 123"));
        assert_eq!(parse_command("/some test  123", "some"), Some("test  123"));

        assert_eq!(parse_command("你好@testbot", "some"), None);
        assert_eq!(parse_command("some@testbot", "some"), None);
        assert_eq!(parse_command(" /some test 123", "some"), None);
        assert_eq!(parse_command("/some@otherbot", "some"), None);
        assert_eq!(parse_command("/some@otherbot 1 2 3", "some"), None);

        assert_eq!(parse_command("/some@testbot", "some"), Some(""));
        assert_eq!(parse_command("/some@testbot ", "some"), Some(""));
        assert_eq!(parse_command("/some@testbot   ", "some"), Some(""));
        assert_eq!(parse_command("/some@testbot   123", "some"), Some("123"));
        assert_eq!(
            parse_command("/some@testbot test 123  ", "some"),
            Some("test 123")
        );
        assert_eq!(
            parse_command("/some@testbot test  123", "some"),
            Some("test  123")
        );
    }

    #[test]
    fn split_n_tests() {
        use crate::utils::split_n;

        assert_eq!(split_n::<3>(""), (vec![], None));
        assert_eq!(split_n::<3>("11 22 33"), (vec!["11", "22"], Some("33")));
        assert_eq!(split_n::<4>("11 22 33"), (vec!["11", "22", "33"], None));
        assert_eq!(
            split_n::<3>("11  22  33  44"),
            (vec!["11", "22"], Some("33  44"))
        );
        assert_eq!(
            split_n::<3>("  11  22  33  44  "),
            (vec!["11", "22"], Some("33  44"))
        );
    }
}
