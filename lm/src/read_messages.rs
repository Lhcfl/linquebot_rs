use std::path::Path;

use anyhow::Context;

#[derive(Debug)]
pub struct MsgBundle(serde_json::Value);

impl MsgBundle {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let res = serde_json::from_reader(std::fs::File::open(path)?)?;
        Ok(Self(res))
    }

    pub fn iter(&self) -> anyhow::Result<impl Iterator<Item = String>> {
        let data = self
            .0
            .as_object()
            .context("result obj")?
            .get("messages")
            .context("messages")?
            .as_array()
            .context("msg array")?;
        Ok(data.into_iter().filter_map(|msg| {
            let text = &msg.as_object().expect("msg obj")["text"];
            if let Some(text) = text.as_str() {
                if text.is_empty() { None } else { Some(format!("{text}{}", super::DELIM)) }
            } else if let Some(text) = text.as_array() {
                let mut res = "".to_string();
                for seg in text {
                    if let Some(raw) = seg.as_str() {
                        res += raw;
                    } else if let Some(obj) = seg.as_object() {
                        let text = obj["text"].as_str().expect("str text");
                        res += &match obj["type"].as_str().expect("str type") {
                            "pre" => format!(
                                "```{}\n{text}\n```",
                                obj["language"].as_str().expect("str lang")
                            ),
                            "code" => format!("`{text}`"),
                            "italic" => format!("__{text}__"),
                            "bold" => format!("**{text}**"),
                            "strikethrough" => format!("~~{text}~~"),
                            _ => text.to_string(),
                        };
                    }
                }
                res += super::DELIM;
                Some(res)
            } else {
                None
            }
        }))
    }
}
