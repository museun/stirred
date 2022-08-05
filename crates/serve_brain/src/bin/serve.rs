use std::{collections::HashMap, path::PathBuf, sync::Arc};

use brain::{spawn_brain, start_server, state::State, Messaging};
use gumdrop::Options;
use markov::Brain;
use tokio::sync::Mutex;

#[derive(Debug, Options)]
struct Config {
    #[options(help = "print this help message")]
    help: bool,

    #[options(help = "directory to store the brains", required)]
    directory: PathBuf,

    #[options(
        help = "address to listen on",
        default = "localhost:50000",
        meta = "addr"
    )]
    address: String,
}

async fn load_brains(paths: &[PathBuf]) -> State {
    let mut map = HashMap::<String, Messaging>::default();
    for name in paths.into_iter() {
        let stem = name.file_stem().expect("valid path");

        let (tx, mut rx) = tokio::sync::mpsc::channel(paths.len());
        tokio::task::spawn({
            let tx = tx.clone();
            let name = name.to_owned();
            let stem = stem.to_owned();
            async move {
                let brain = brain::load(&name)
                    .await
                    .unwrap_or_else(|_| Brain::new(stem.to_string_lossy(), 5));
                let _ = tx.send((name, brain)).await;
            }
        });
        drop(tx);

        while let Some((name, brain)) = rx.recv().await {
            let out = spawn_brain(brain, name);
            map.insert(stem.to_string_lossy().to_string(), Messaging::new(out));
        }
    }

    State {
        brains: Arc::new(Mutex::new(map)),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse_args_default_or_exit();

    let mut paths = vec![];
    let mut stream = tokio::fs::read_dir(config.directory).await?;
    while let Some(entry) = stream.next_entry().await.ok().flatten() {
        let path = entry.path();
        if let Some("sdb") = path.extension().and_then(|s| s.to_str()) {
            paths.push(path)
        }
    }

    let brains = load_brains(&paths).await;
    start_server(config.address, brains).await
}
