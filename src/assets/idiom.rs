use rand::{seq::IteratorRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};

use crate::utils::escape_html;
const IDIOM_STR: &str = include_str!("idiom.json");

#[derive(Debug, Serialize, Deserialize)]
pub struct Idiom {
    pub derivation: String,
    pub example: String,
    pub explanation: String,
    pub pinyin: String,
    pub word: String,
    pub abbreviation: String,
    pub pinyin_r: String,
    pub first: String,
    pub last: String,
}

impl Idiom {
    pub fn html_description(&self) -> String {
        format!(
            "<b>{}</b> ({}): {}\n来源：{}\n使用例：{}",
            escape_html(&self.word),
            escape_html(&self.pinyin),
            escape_html(&self.explanation),
            escape_html(&self.derivation),
            escape_html(&self.example)
        )
    }
}

static IDIOMS: LazyLock<Vec<Idiom>> = LazyLock::new(|| serde_json::from_str(IDIOM_STR).unwrap());
static IDIOM_MAP: LazyLock<HashMap<String, &'static Idiom>> = LazyLock::new(|| {
    let mut map: HashMap<String, &'static Idiom> = HashMap::new();
    for idiom in IDIOMS.iter() {
        map.insert(idiom.word.clone(), idiom);
    }
    map
});

pub fn random_idiom() -> &'static Idiom {
    IDIOMS.iter().choose(&mut thread_rng()).unwrap()
}

pub fn get_idiom(word: &str) -> Option<&'static Idiom> {
    IDIOM_MAP.get(word).map(|idiom| *idiom)
}
