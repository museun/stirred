use std::{borrow::Borrow, collections::HashMap, hash::Hash, str::FromStr};

use anyhow::Context;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Tags {
    pub(crate) map: HashMap<Box<str>, Box<str>>,
}

impl Tags {
    pub fn parse(input: &mut &str) -> Option<Self> {
        if !input.starts_with('@') {
            return None;
        }

        let (head, tail) = input.split_once(' ')?;
        *input = tail;

        let map = head[1..]
            .split(';')
            .flat_map(|s| s.split_once('='))
            .map(|(k, v)| (Box::from(k.trim()), Box::from(v.trim())))
            .collect();

        Some(Self { map })
    }

    pub fn get<K>(&self, k: &K) -> anyhow::Result<&str>
    where
        K: Hash + Eq + ?Sized + std::fmt::Display,
        Box<str>: Borrow<K>,
    {
        self.map
            .get(k)
            .map(|s| &**s)
            .with_context(|| format!("cannot find: {k}"))
    }

    pub fn get_parsed<K, T>(&self, k: &K) -> anyhow::Result<T>
    where
        K: Hash + Eq + ?Sized + std::fmt::Display,
        Box<str>: Borrow<K>,
        T: FromStr,
        T::Err: Into<anyhow::Error>,
    {
        self.get(k)?.parse().map_err(Into::into)
    }
}
