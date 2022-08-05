use std::sync::Arc;

#[allow(dead_code)]
pub fn defer(f: impl FnMut()) -> impl Drop {
    struct Defer<F: FnMut()>(Option<F>);
    impl<F: FnMut()> Drop for Defer<F> {
        fn drop(&mut self) {
            if let Some(mut f) = self.0.take() {
                f()
            }
        }
    }
    Defer(Some(f))
}

pub trait VecExt<T> {
    fn append_maybe(&mut self, right: Self);
}

impl<T> VecExt<T> for Vec<T> {
    fn append_maybe(&mut self, mut right: Self) {
        if right.is_empty() {
            return;
        }
        if self.is_empty() {
            std::mem::swap(self, &mut right);
            return;
        }
        self.append(&mut right)
    }
}

pub trait IterExt<T>: Iterator<Item = T> + Sized
where
    T: AsRef<str>,
{
    fn join_with(self, ch: char) -> String {
        self.fold(String::new(), |mut a, c| {
            if !a.is_empty() {
                a.push(ch)
            }
            a.push_str(c.as_ref());
            a
        })
    }
}

impl<T, I> IterExt<T> for I
where
    T: AsRef<str>,
    I: Iterator<Item = T>,
{
}

#[allow(dead_code)]
pub fn into_clone<T>(d: Arc<T>) -> T
where
    T: Clone,
{
    Arc::try_unwrap(d).unwrap_or_else(|t| (*t).clone())
}

#[derive(Copy, Clone)]
pub enum Quote<T> {
    Single(T),
    Double(T),
    Tick(T),
}

impl<T> Quote<T> {
    pub const fn inner(&self) -> &T {
        match self {
            Self::Single(inner) | Self::Double(inner) | Self::Tick(inner) => inner,
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Quote<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(s) => write!(f, "'{s}'"),
            Self::Double(s) => write!(f, "\"{s}\""),
            Self::Tick(s) => write!(f, "`{s}`"),
        }
    }
}

impl<T> AsRef<T> for Quote<T> {
    fn as_ref(&self) -> &T {
        self.inner()
    }
}

impl<T> std::ops::Deref for Quote<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(s) | Self::Double(s) | Self::Tick(s) => s,
        }
    }
}

impl From<Quote<&str>> for String {
    fn from(q: Quote<&str>) -> Self {
        (*q.inner()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_quote() {
        let _g = crate::testing::insta_settings("quote_single");
        let s = Quote::Single("hello world");
        insta::assert_display_snapshot!(s, @"'hello world'");
    }

    #[test]
    fn double_quote() {
        let _g = crate::testing::insta_settings("quote_double");
        let s = Quote::Double("hello world");
        insta::assert_display_snapshot!(s, @r###""hello world""###);
    }

    #[test]
    fn backtick_quote() {
        let _g = crate::testing::insta_settings("quote_tick");
        let s = Quote::Tick("hello world");
        insta::assert_display_snapshot!(s, @"`hello world`");
    }
}
