use anyhow::Context;
use axum::{
    routing::{get, post},
    Extension, Router, Server,
};
use futures::{SinkExt, StreamExt};
use markov::Brain;
use std::{path::Path, time::Duration};

pub mod state;

mod handlers;

mod messaging;
pub use messaging::{spawn_brain, Messaging};

mod request;
mod response;

pub const SAVE_DURATION: Duration = Duration::from_secs(5 * 60);
pub const GENERATE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn start_server(
    addr: impl tokio::net::ToSocketAddrs + Send + 'static,
    state: state::State,
) -> anyhow::Result<()> {
    let addr = tokio::net::lookup_host(addr)
        .await?
        .next()
        .with_context(|| "could not resolve an addr")?;

    let app = Router::new()
        .route("/:name/generate", get(handlers::generate))
        .route("/:name/train", post(handlers::train))
        .route("/:name/create", post(handlers::create))
        .layer(Extension(state));

    Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

pub async fn load(path: impl AsRef<Path> + Send) -> anyhow::Result<Brain> {
    let reader = tokio::io::BufReader::new(tokio::fs::File::open(path).await?);

    let dec = async_compression::tokio::bufread::ZstdDecoder::new(reader);
    let mut reader = async_bincode::tokio::AsyncBincodeReader::from(dec);

    let element: Brain = reader
        .next()
        .await
        .with_context(|| "cannot deserialize brain")??;

    Ok(element)
}

pub async fn save(brain: &Brain, path: impl AsRef<Path> + Send) -> anyhow::Result<()> {
    let writer = tokio::io::BufWriter::new(tokio::fs::File::create(path).await?);
    let enc = async_compression::tokio::write::ZstdEncoder::new(writer);
    let mut writer = async_bincode::tokio::AsyncBincodeWriter::from(enc);
    writer.send(brain).await?;
    Ok(())
}

pub fn save_sync(brain: &Brain, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    let enc = zstd::Encoder::new(writer, 0)?;
    bincode::serialize_into(enc, brain)?;
    Ok(())
}

pub trait BrainExt {
    fn save(&self, path: &Path) -> anyhow::Result<()>;
}

impl BrainExt for Brain {
    fn save(&self, path: &Path) -> anyhow::Result<()> {
        save_sync(self, path)
    }
}
