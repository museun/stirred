use std::path::Path;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

pub trait PersistFormat {
    fn serialize<'a, T: Sync, W>(data: &'a T, out: &'a mut W) -> BoxFuture<'a, anyhow::Result<()>>
    where
        T: ::serde::Serialize,
        W: tokio::io::AsyncWrite + Unpin + Send + ?Sized;

    fn deserialize<'a, T, R>(input: &'a mut R) -> BoxFuture<'a, anyhow::Result<T>>
    where
        T: Send + for<'de> ::serde::Deserialize<'de>,
        R: tokio::io::AsyncRead + Unpin + Send + ?Sized;

    fn ext() -> &'static str;
}

pub struct Json;

impl PersistFormat for Json {
    fn serialize<'a, T: Sync, W>(data: &'a T, out: &'a mut W) -> BoxFuture<'a, anyhow::Result<()>>
    where
        T: serde::Serialize,
        W: tokio::io::AsyncWrite + Unpin + Send + ?Sized,
    {
        Box::pin(async {
            let data = serde_json::to_vec(data)?;
            Ok(out.write_all(&data).await?)
        })
    }

    fn deserialize<'a, T, R>(input: &'a mut R) -> BoxFuture<'a, anyhow::Result<T>>
    where
        T: Send + for<'de> serde::Deserialize<'de>,
        R: tokio::io::AsyncRead + Unpin + Send + ?Sized,
    {
        Box::pin(async {
            let mut out = String::new();
            input.read_to_string(&mut out).await?;
            Ok(serde_json::from_str(&out)?)
        })
    }

    fn ext() -> &'static str {
        "json"
    }
}

pub trait Persist
where
    for<'de> Self: ::serde::Serialize + ::serde::Deserialize<'de>,
{
    fn save<'a, K: PersistFormat>(
        &'a self,
        out: &'a mut (dyn tokio::io::AsyncWrite + Unpin + Send),
    ) -> BoxFuture<'a, anyhow::Result<()>>
    where
        Self: Send + Sync,
    {
        Box::pin(async { K::serialize(self, out).await })
    }

    fn load<'a, K: PersistFormat>(
        input: &'a mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> BoxFuture<'a, anyhow::Result<Self>>
    where
        Self: Sized + Send + 'a,
    {
        Box::pin(async { K::deserialize(input).await })
    }
}

impl<T: for<'de> ::serde::Deserialize<'de> + ::serde::Serialize> Persist for T {}

pub trait PersistExt: Persist + Send + Sync {
    fn save_to_file<'a, K: PersistFormat>(
        &'a self,
        path: &'a (dyn AsRef<Path> + Send + Sync + 'a),
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut file = tokio::fs::File::create(path).await?;
            self.save::<K>(&mut file).await
        })
    }

    fn load_from_file<'a, K: PersistFormat>(
        path: &'a (dyn AsRef<Path> + Send + Sync + 'a),
    ) -> BoxFuture<'a, anyhow::Result<Self>> {
        Box::pin(async move {
            let mut file = tokio::fs::File::open(path).await?;
            Self::load::<K>(&mut file).await
        })
    }
}

impl<T: Send + Sync + 'static> PersistExt for T where T: Persist {}
