use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Context;
use markov::Brain;
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{request, BrainExt, GENERATE_TIMEOUT, SAVE_DURATION};

#[derive(Clone)]
pub struct Messaging {
    tx: Sender<(Request, oneshot::Sender<Response>)>,
}

impl Messaging {
    pub const fn new(tx: Sender<(Request, oneshot::Sender<Response>)>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, req: Request) -> Response {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send((req, tx)).await;
        rx.await.unwrap()
    }
}

pub enum Response {
    Generated { data: String },
    Error { error: anyhow::Error },
    Nothing,
}

pub enum Request {
    Train { data: String },
    Generate { opts: request::Generate },
    Save,
    ForceSave,
}

pub fn spawn_brain(
    mut brain: Brain,
    path: impl Into<PathBuf>,
) -> Sender<(Request, oneshot::Sender<Response>)> {
    use {Request as In, Response as Out};

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(In, oneshot::Sender<Out>)>(16);
    let mut last = Instant::now();
    let path = path.into();

    let func = move || {
        while let Some((msg, out)) = rx.blocking_recv() {
            let mut out = Some(out);
            let resp = match handle_message(msg, &mut brain, &mut out, &mut last, &path) {
                Ok(false) => Response::Nothing,
                Err(error) => Response::Error { error },
                Ok(true) => continue,
            };
            let _ = out.take().unwrap().send(resp);
        }
    };

    let _join = std::thread::spawn(func);
    tx
}

fn handle_message(
    msg: Request,
    brain: &mut Brain,
    out: &mut Option<oneshot::Sender<Response>>,
    last: &mut Instant,
    path: &Path,
) -> anyhow::Result<bool> {
    use Request::*;
    use Response::*;

    let mut sent = false;
    let mut send = |msg| {
        let _ = out.take().unwrap().send(msg);
        sent = true
    };

    match msg {
        Train { data } => train(brain, &data),

        Generate { opts } => {
            let resp = generate(brain, opts)
                .map(|data| Generated { data })
                .with_context(|| "cannot generate data")?;
            send(resp);
        }

        ForceSave => {
            save(brain, path)?;
            *last = Instant::now();
        }

        Save if last.elapsed() >= SAVE_DURATION => {
            save(brain, path)?;
            *last = Instant::now();
        }

        Save => {}
    }

    Ok(sent)
}

fn generate(brain: &Brain, opts: request::Generate) -> Option<String> {
    brain.generate(
        &fastrand::Rng::new(),
        opts.min,
        opts.max,
        opts.query.as_deref(),
        GENERATE_TIMEOUT,
    )
}

fn train(brain: &mut Brain, data: &str) {
    brain.train(data)
}

fn save(brain: &Brain, path: &Path) -> anyhow::Result<()> {
    brain.save(path)
}
