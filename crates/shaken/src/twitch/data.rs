use std::collections::{HashMap, HashSet};

#[derive(::serde::Deserialize)]
pub struct Data<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub template: String,
}

#[derive(Clone, Debug, ::serde::Deserialize)]
pub struct Stream {
    #[serde(deserialize_with = "crate::serde::from_str")]
    pub id: u64,

    #[serde(deserialize_with = "crate::serde::from_str")]
    pub user_id: u64,
    pub user_name: String,

    #[serde(deserialize_with = "crate::serde::from_str")]
    pub game_id: u64,
    pub title: String,
    pub viewer_count: u64,

    #[serde(deserialize_with = "crate::serde::assume_utc_date_time")]
    pub started_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, ::serde::Deserialize)]
pub struct Emote {
    pub id: String,
    pub name: String,
}

// data	object array
//    id	string
//    name	string
//    images	object
//       url_1x	string
//       url_2x	string
//       url_4x	string
//    format	string array
//    scale	string array
//    theme_mode	string array
// template	string

#[derive(Clone, Default)]
pub struct EmoteMap {
    name_to_id: HashMap<Box<str>, Box<str>>,
    id_to_name: HashMap<Box<str>, Box<str>>,
    names: HashSet<Box<str>>,
}

impl EmoteMap {
    pub fn with_emotes<'k, 'v>(mut self, iter: impl Iterator<Item = (&'k str, &'v str)>) -> Self {
        for (k, v) in iter {
            self.name_to_id.insert(v.into(), k.into());
            self.name_to_id.insert(k.into(), v.into());
            self.names.insert(k.into());
        }
        self
    }

    pub fn get_name(&self, id: &str) -> Option<&str> {
        self.id_to_name.get(id).map(|s| &**s)
    }

    pub fn get_id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|s| &**s)
    }

    pub fn has(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> + ExactSizeIterator + '_ {
        self.names.iter().map(|s| &**s)
    }
}
