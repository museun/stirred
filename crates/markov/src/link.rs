use std::cmp::Ordering;

use super::Token;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Link {
    pub token: Token,
    pub count: usize,
}

impl Link {
    #[inline(always)]
    pub const fn new(token: Token) -> Self {
        Self { token, count: 1 }
    }

    #[inline(always)]
    pub fn merge(&mut self, other: &Self) {
        // TODO this should actually merge tokens
        debug_assert!(other.token == self.token);
        self.count += other.count;
    }

    #[inline(always)]
    pub fn expand(&mut self, count: usize) {
        self.count += count
    }
}

impl PartialOrd for Link {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.count.partial_cmp(&other.count)
    }
}

impl Ord for Link {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.count.cmp(&other.count)
    }
}

impl PartialEq for Link {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.count.eq(&other.count)
    }
}

impl Eq for Link {}
