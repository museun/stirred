use super::{Link, Token};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Set(pub(crate) Vec<Link>); // TODO a small vec

impl Set {
    pub fn new(token: Token) -> Self {
        Self(vec![Link::new(token)])
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn insert(&mut self, token: Token) {
        if let Some(existing) = self.find_mut(&token) {
            existing.expand(1);
            // TODO this doesn't have to be done at this stage
            // self.0.sort_unstable_by(|a, b| a.cmp(b).reverse());
            // self.0.dedup();
            return;
        }

        self.0.push(Link::new(token))
    }

    fn find_mut(&mut self, token: &Token) -> Option<&mut Link> {
        self.0.iter_mut().find(|left| &left.token == token)
    }
}
