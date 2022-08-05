use std::time::Duration;

use fastrand_ext::IterExt as _;
use hashbrown::{HashMap, HashSet};

use crate::{Link, Set, Token, Word};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Brain {
    name: String,
    depth: usize,
    chain: HashMap<Vec<Word>, Set>,
    head: HashSet<Word>,
}

impl Brain {
    pub fn new(name: impl Into<String>, depth: usize) -> Self {
        Self {
            name: name.into(),
            depth,
            chain: HashMap::default(),
            head: HashSet::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }

    #[tracing::instrument(skip(self, rng))]
    pub fn generate(
        &self,
        rng: &fastrand::Rng,
        min: usize,
        max: usize,
        query: Option<&str>,
        time_out: Duration,
    ) -> Option<String> {
        let mut words = <Vec<Word>>::new();
        let mut indices = Adjacent::new();

        let mut base = Self::base_words(query);
        rng.shuffle(&mut base);

        let mut pick = |max: usize| loop {
            let t = if max == 1 { max } else { rng.usize(1..max) };
            if indices.create_adjacency(t) {
                break t;
            }
        };

        let mut choose = |words: &mut Vec<Word>| {
            if !base.is_empty() && words.len() > 1 && rng.f64() > rng.f64() {
                let next = base.pop().unwrap();
                let n = pick(words.len());
                words.insert(n, next) // TODO this would be better as a linked list
            }
        };

        let now = std::time::Instant::now();
        'outer: loop {
            if now.elapsed() > time_out {
                return None;
            }

            if words.len() >= min {
                break;
            }

            choose(&mut words);
            words.push(self.head.iter().choose(rng)?.clone());
            if words.len() >= max {
                break;
            }

            while let Token::Word(word) =
                self.select_token(rng, Self::context(words.as_slice(), self.depth))
            {
                choose(&mut words);
                words.push(word);
                if words.len() >= max {
                    break 'outer;
                }
            }
        }

        while let Some(word) = base.pop() {
            let n = pick(words.len());
            words.insert(n, word)
        }

        let capacity = words.iter().map(|s| s.len() + 1).sum();
        let mut out = String::with_capacity(capacity);

        for word in words.into_iter().take(max) {
            if let Ok(word) = std::str::from_utf8(&word) {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(word);
            }
        }

        out.shrink_to_fit();
        Some(out)
    }

    #[tracing::instrument(skip(self))]
    pub fn train(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }

        let words = text
            .split_whitespace()
            .filter_map(|s| (!s.is_empty()).then(|| s.as_bytes().into()))
            .collect::<Vec<Word>>();

        self.head.insert(words[0].clone());

        let depth = std::cmp::min(self.depth, words.len() - 1);
        for width in 1..=depth {
            // TODO take ownership to remove the extra clone
            for window in words.windows(width + 1) {
                let tail = window.last().cloned().unwrap();
                self.train_link(&window[..window.len() - 1], Token::Word(tail));
            }
            self.train_link(&words[words.len() - width..], Token::End)
        }
    }

    #[tracing::instrument(skip(self))]
    fn train_link(&mut self, context: &[Word], token: Token) {
        use hashbrown::hash_map::RawEntryMut::*;
        match self.chain.raw_entry_mut().from_key(context) {
            Occupied(set) => {
                set.into_mut().insert(token);
            }
            Vacant(e) => {
                e.insert(context.to_vec(), Set::new(token));
            }
        };
    }

    #[tracing::instrument(skip(self, rng))]
    fn select_token(&self, rng: &fastrand::Rng, context: &[Word]) -> Token {
        let upper = std::cmp::min(self.depth, context.len());
        let mut sets = (1..=upper)
            .filter_map(|w| {
                self.chain
                    .get(&context[context.len() - w..])
                    .map(|set| (w, set.clone()))
            })
            .peekable();

        let mut links: Vec<Link> = match sets.peek() {
            Some((_, set)) => Vec::with_capacity(set.size()),
            None => return Token::End,
        };

        for (width, set) in sets {
            for mut link in set.0 {
                link.count *= width;
                match links.iter_mut().find(|left| left.token == link.token) {
                    Some(e) => e.merge(&link),
                    None => links.push(link),
                }
            }
        }

        Self::weighted_select(&links, rng).token.clone()
    }

    fn weighted_select<'a>(links: &'a [Link], rng: &fastrand::Rng) -> &'a Link {
        let mut sum = links.iter().map(|Link { count, .. }| count).sum();
        for link in links.iter().cycle().skip(rng.usize(0..sum)) {
            match sum.checked_sub(link.count) {
                Some(n) => sum = n,
                None => return link,
            }
        }
        unreachable!("DAG illformed")
    }

    #[tracing::instrument]
    fn base_words(input: Option<&str>) -> Vec<Word> {
        input
            .map(|data| {
                data.split_ascii_whitespace()
                    .map(|s| s.bytes().collect())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[inline(always)]
    fn context(words: &[Word], depth: usize) -> &[Word] {
        &words[words.len().saturating_sub(depth)..]
    }
}

struct Adjacent(Vec<usize>);
impl Adjacent {
    const fn new() -> Self {
        Self(Vec::new())
    }
}

impl Adjacent {
    fn create_adjacency(&mut self, index: usize) -> bool {
        if self.0.contains(&index) {
            return false;
        }

        self.0.reserve(0);
        for index in [index.saturating_sub(1), index, index + 1] {
            self.0.push(index)
        }
        true
    }
}
