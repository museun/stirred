#[derive(Debug)]
pub struct DontCare;

impl std::fmt::Display for DontCare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DontCare")
    }
}

impl std::error::Error for DontCare {}

pub trait ErrorExt<T> {
    fn dont_care(self) -> anyhow::Result<T>;
}

impl<T> ErrorExt<T> for Option<T> {
    fn dont_care(self) -> anyhow::Result<T> {
        match self {
            Some(ok) => Ok(ok),
            None => dont_care(),
        }
    }
}

impl<T, E> ErrorExt<T> for Result<T, E>
where
    E: Send + std::fmt::Display,
{
    fn dont_care(self) -> anyhow::Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(..) => dont_care(),
        }
    }
}

pub fn dont_care<T>() -> anyhow::Result<T> {
    Err(DontCare.into())
}
