pub mod base64;
pub mod pattern;

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

static TG_SANITIZER: LazyLock<ammonia::Builder> = LazyLock::new(|| {
    let mut builder = ammonia::Builder::empty();
    builder.tags(HashSet::from([
        "b",
        "strong",
        "i",
        "em",
        "u",
        "ins",
        "s",
        "strike",
        "del",
        "span",
        "tg-spoiler",
        "a",
        "tg-emoji",
        "code",
        "pre",
        "blockquote",
    ]));
    let mut tag_attrs = HashMap::<&'static str, HashSet<&'static str>>::new();
    tag_attrs.insert("a", HashSet::from(["href"]));
    builder.tag_attributes(tag_attrs);
    builder
});

pub fn sanitize_html(str: &str) -> String {
    TG_SANITIZER.clean(str).to_string()
}

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

pub fn split_n<const N: usize>(src: &str) -> (Vec<&str>, Option<&str>) {
    let mut it = src.split(|c: char| c.is_whitespace());
    let pre = (1..N)
        .map_while(|_| it.find(|s| !s.is_empty()))
        .collect::<Vec<_>>();
    (pre, it.remainder().map(|str| str.trim()))
}

pub fn split_args<const N: usize>(src: &str) -> [&str; N] {
    let mut res: [&str; N] = [""; N];
    let mut it = src.split(|c: char| c.is_whitespace());
    let mut idx = 0;

    (1..N)
        .map_while(|_| it.find(|s| !s.is_empty()))
        .for_each(|ctnt| {
            res[idx] = ctnt;
            idx += 1;
        });

    res[idx] = it.remainder().unwrap_or("").trim();
    res
}

pub fn partition_results<const N: usize, T, E>(input: [Result<T, E>; N]) -> Result<[T; N], Vec<E>> {
    let mut oks: [Option<T>; N] = [(); N].map(|_| None);
    let mut errs: Vec<E> = Vec::new();
    for (i, item) in input.into_iter().enumerate() {
        match item {
            Ok(v) => oks[i] = Some(v),
            Err(e) => errs.push(e),
        }
    }
    if errs.is_empty() {
        // SAFETY: we have ensured all elements in `oks` are `Some`
        Ok(oks.map(|o| o.unwrap()))
    } else {
        Err(errs)
    }
}

pub mod telegram {
    pub mod prelude {
        use std::future::Future;

        use log::warn;
        pub trait WarnOnError {
            async fn warn_on_error(self, name: &str);
        }

        impl<T, R, E> WarnOnError for T
        where
            T: Future<Output = Result<R, E>> + Send,
            E: ToString,
        {
            async fn warn_on_error(self, name: &str) {
                let res = self.await;
                if let Err(err) = res {
                    warn!(target: name, "Error: {}", err.to_string())
                }
            }
        }
        use reqwest::Response;
        use teloxide_core::{
            types::{ChatId, Message, User},
            Bot,
        };

        pub trait UserExtension {
            fn html_link(&self) -> String;
        }

        impl UserExtension for User {
            fn html_link(&self) -> String {
                format!("<a href=\"{}\">{}</a>", self.url(), self.full_name())
            }
        }

        pub trait MessageExtension {
            fn is_reply_to_channel(&self) -> bool;
        }

        impl MessageExtension for Message {
            fn is_reply_to_channel(&self) -> bool {
                self.reply_to_message()
                    .and_then(|msg| msg.sender_chat.as_ref())
                    .is_some_and(|chat| chat.is_channel())
            }
        }

        pub trait BotExtension {
            async fn send_sticker_by_file_id(
                &self,
                chat_id: ChatId,
                file_id: &str,
            ) -> reqwest::Result<Response>;
        }

        impl BotExtension for Bot {
            async fn send_sticker_by_file_id(
                &self,
                chat_id: ChatId,
                file_id: &str,
            ) -> reqwest::Result<Response> {
                reqwest::get(format!(
                    "{}bot{}/sendSticker?chat_id={}&sticker={}",
                    self.api_url(),
                    self.token(),
                    chat_id,
                    file_id
                ))
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn split_args_test() {
        use crate::utils::split_args;
        assert_eq!(split_args::<3>(""), ["", "", ""]);
        assert_eq!(split_args::<3>("11 22 33"), ["11", "22", "33"]);
        assert_eq!(split_args::<4>("11 22 33"), ["11", "22", "33", ""]);
        assert_eq!(split_args::<3>("11  22  33  44"), ["11", "22", "33  44"]);
        assert_eq!(
            split_args::<3>("  11  22  33  44  "),
            ["11", "22", "33  44"]
        );
    }
}
