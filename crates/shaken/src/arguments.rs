use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use anyhow::Context as _;

#[derive(Default, Debug)]
//
#[derive(Clone)] // TODO remove this
pub struct Arguments {
    // TODO borrow this from the input
    pub map: HashMap<String, String>,
}

impl Arguments {
    pub fn get(&self, key: &str) -> anyhow::Result<&str> {
        self.map
            .get(key)
            .map(|s| &**s)
            .with_context(|| format!("cannot find {key}"))
    }

    pub fn get_parsed<T>(&self, key: &str) -> anyhow::Result<T>
    where
        T: FromStr,
        T::Err: Send + Sync + 'static,
        T::Err: std::error::Error,
        T::Err: Into<anyhow::Error>,
    {
        Ok(self.get(key)?.parse()?)
    }
}

impl std::ops::Index<&str> for Arguments {
    type Output = str;

    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).unwrap()
    }
}

#[derive(Debug)]
pub enum Match<T> {
    Required,
    Match(T),
    NoMatch,
    Exact,
}

#[derive(Default, Debug)]
pub struct ExampleArgs {
    pub args: Box<[ArgType]>,
}

impl ExampleArgs {
    pub fn contains(&self, arg: &Kind) -> bool {
        self.args.iter().any(|ArgType { kind, .. }| kind == arg)
    }
}

impl ExampleArgs {
    const REQUIRED: Kind = Kind::Required;
    const OPTIONAL: Kind = Kind::Optional;
    const VARIADIC: Kind = Kind::Variadic;
    pub fn extract(&self, mut input: &str) -> Match<HashMap<String, String>> {
        if input.is_empty() {
            if self.contains(&Self::REQUIRED) {
                return Match::Required;
            }
            if !self.args.is_empty()
                && (!self.contains(&Self::OPTIONAL) && !self.contains(&Self::VARIADIC))
            {
                return Match::NoMatch;
            }
            if self.args.is_empty() {
                return Match::Exact;
            }
        }

        if !input.is_empty() && self.args.is_empty() {
            return Match::NoMatch;
        }

        use Kind::*;
        let mut map = HashMap::new();
        for ArgType { key, kind } in &*self.args {
            match (kind, input.find(' ')) {
                (Required | Optional, None) | (Variadic, ..) => {
                    if !input.is_empty() {
                        map.insert(key.into(), input.into());
                    }
                    break;
                }
                (.., Some(pos)) => {
                    let (head, tail) = input.split_at(pos);
                    map.insert(key.into(), head.into());
                    input = tail.trim();
                }
            }
        }

        Match::Match(map)
    }

    pub fn parse(input: &str) -> anyhow::Result<Self> {
        // <required> <optional?> <rest..>
        let mut seen = HashSet::new();
        let mut args = vec![];

        for token in input.split_ascii_whitespace() {
            let mut append = |arg: &[_]| {
                let data = &token[1..arg.len() + 1];
                anyhow::ensure!(seen.insert(data), "{data} was already used");
                Ok(data.into())
            };

            let all_alpha = |s: &[u8]| s.iter().all(|c| c.is_ascii_alphabetic());

            let arg = match token.as_bytes() {
                [b'<', arg @ .., b'.', b'.', b'>'] if all_alpha(arg) => ArgType {
                    key: append(arg)?,
                    kind: Kind::Variadic,
                },
                [b'<', arg @ .., b'?', b'>'] if all_alpha(arg) => ArgType {
                    key: append(arg)?,
                    kind: Kind::Optional,
                },
                [b'<', arg @ .., b'>'] if all_alpha(arg) => ArgType {
                    key: append(arg)?,
                    kind: Kind::Required,
                },
                // TODO report invalid patterns
                // TODO report invalid characters in keys
                _ => continue,
            };

            args.push(arg);
            if matches!(
                args.last(),
                Some(&ArgType {
                    kind: Kind::Variadic,
                    ..
                })
            ) {
                break;
            }
        }

        Ok(Self { args: args.into() })
    }
}

#[derive(Debug)]
pub struct ArgType {
    key: String,
    kind: Kind,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Required,
    Optional,
    Variadic,
}
