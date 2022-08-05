use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    error::DontCare,
    irc::{Privmsg, Tags},
    state::SharedState,
    twitch,
    util::VecExt,
    Arguments,
};

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type BoxedResponse = BoxedFuture<'static, anyhow::Result<Response>>;
pub type BoxedCallable<A = Request, B = anyhow::Result<Response>, C = BoxedResponse> =
    Arc<dyn Callable<A, B, Out = C> + Sync>;

pub trait Callable<A, B>
where
    Self: Send + Sync,
    A: Send + 'static,
    B: Send + 'static,
{
    type Out: Future<Output = B>;

    fn call(&self, req: A) -> Self::Out;

    fn command_names(&self) -> Vec<&str> {
        vec![]
    }

    fn all_usage_and_help(&self) -> Vec<(&str, &str)> {
        vec![]
    }

    fn usage(&self) -> Option<&str> {
        None
    }

    fn help(&self) -> Option<&str> {
        None
    }
}

impl<const N: usize> Callable<Request, anyhow::Result<Response>> for [BoxedCallable; N] {
    type Out = BoxedResponse;

    fn call(&self, req: Request) -> Self::Out {
        let (tx, mut rx) = tokio::sync::mpsc::channel(self.len());
        for (i, callable) in self.iter().map(Arc::clone).enumerate() {
            let req = req.clone();
            let _ = tokio::spawn({
                let tx = tx.clone();
                async move {
                    let res = callable.call(req).await;

                    if let Ok(r) = &res {
                        if r.is_empty() {
                            return;
                        }
                    }

                    let _ = tx.send((i, res)).await;
                    log::trace!("end of callable callable: {i}");
                }
            });
        }

        drop(tx);

        Box::pin(async move {
            // TODO build this with the msg-id
            let mut resp = Response::empty();
            while let Some((i, r)) = rx.recv().await {
                match r {
                    Ok(right) => {
                        if !right.is_empty() {
                            log::trace!("got response for: {i}");
                        }
                        resp.kind.append_maybe(right.kind);
                    }
                    Err(err) if !err.is::<DontCare>() => {
                        resp = resp.problem(err.to_string());
                    }
                    _ => {}
                };
            }
            Ok(resp)
        })
    }

    fn command_names(&self) -> Vec<&str> {
        self.iter().flat_map(|c| c.command_names()).collect()
    }

    fn all_usage_and_help(&self) -> Vec<(&str, &str)> {
        self.iter().flat_map(|c| c.all_usage_and_help()).collect()
    }
}

impl<T, A, B> Callable<A, B> for Arc<T>
where
    T: Callable<A, B> + Send,
    A: Send + 'static,
    B: Send + 'static,
{
    type Out = T::Out;

    fn call(&self, req: A) -> Self::Out {
        std::ops::Deref::deref(self).call(req)
    }
}

impl<A, B, F, Fut> Callable<A, B> for F
where
    F: Fn(A) -> Fut + Send + Sync,
    Fut: Future<Output = B> + Send,
    A: Send + 'static,
    B: Send + 'static,
{
    type Out = Fut;

    fn call(&self, req: A) -> Self::Out {
        (self)(req)
    }
}

#[derive(Clone)]
pub struct Request {
    pub state: SharedState,
    pub tags: Arc<Tags>,
    pub sender: Arc<str>,
    pub target: Arc<str>,
    pub data: Arc<str>,
    pub args: Arguments,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            state: Default::default(),
            tags: Default::default(),
            sender: Arc::from(""),
            target: Arc::from(""),
            data: Arc::from(""),
            args: Default::default(),
        }
    }
}

impl Request {
    pub fn from_pm(state: SharedState, pm: Privmsg) -> Self {
        Request {
            state,
            tags: Arc::new(pm.tags),
            sender: pm.user,
            target: pm.target,
            data: pm.data,
            args: Arguments::default(),
        }
    }

    pub const fn empty(&self) -> Response {
        Response::empty()
    }

    pub fn reply(&self, data: impl Into<Box<str>>) -> Response {
        Response {
            kind: vec![ResponseKind::Reply(data.into())],
        }
    }

    pub fn say(&self, data: impl Into<Box<str>>) -> Response {
        Response {
            kind: vec![ResponseKind::Say(data.into())],
        }
    }

    pub fn problem(&self, data: impl Into<Box<str>>) -> Response {
        Response {
            kind: vec![ResponseKind::Problem(data.into())],
        }
    }

    pub fn streamer_name(&self) -> anyhow::Result<&str> {
        Ok("museun") // TODO get the actual broadcaster name (target, or lookup room-id)
    }

    pub async fn require_streaming(&self, channel: &str) -> anyhow::Result<()> {
        let client = self.state.get::<twitch::HelixClient>().await;
        if let Ok([_stream]) = client.get_streams([channel]).await.as_deref() {
            return Ok(());
        }
        anyhow::bail!("{channel} is not streaming")
    }

    pub fn require_moderator(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.is_from_moderator(),
            "that requires you to be a moderator"
        );
        Ok(())
    }

    pub fn require_broadcaster(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.is_from_broadcaster(),
            "that requires you to be the broadcaster"
        );
        Ok(())
    }

    pub fn require_elevation(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.is_from_moderator() || self.is_from_broadcaster(),
            "that requires you to be a moderator or the broadcaster"
        );
        Ok(())
    }

    pub fn is_from_broadcaster(&self) -> bool {
        self.badge_iter()
            .any(|(key, val)| key == "broadcaster" && val == "1")
    }

    pub fn is_from_moderator(&self) -> bool {
        self.badge_iter()
            .any(|(key, val)| key == "moderator" && val == "1")
    }

    pub fn badge_iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.tags
            .get("badges")
            .into_iter()
            .flat_map(|s| s.split(','))
            .flat_map(|s| s.split_once('/'))
    }

    pub fn split_command(input: &str) -> &str {
        input
            .split_once(' ')
            .map(|(k, _)| k)
            .unwrap_or_else(|| input)
    }

    pub fn match_command(&self, right: &str) -> bool {
        self.command() == Self::split_command(right)
    }

    pub fn command(&self) -> &str {
        Self::split_command(self.data())
    }

    pub fn data(&self) -> &str {
        &self.data
    }
}

#[derive(Debug, ::serde::Serialize)]
pub struct Response {
    pub kind: Vec<ResponseKind>,
}

impl Response {
    pub const fn empty() -> Self {
        Self { kind: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    pub const fn nothing() -> anyhow::Result<Self> {
        Ok(Self::empty())
    }

    pub const fn ok(self) -> anyhow::Result<Self> {
        Ok(self)
    }

    pub fn reply(self, data: impl Into<Box<str>>) -> Self {
        self.append(ResponseKind::Reply(data.into()))
    }

    pub fn say(self, data: impl Into<Box<str>>) -> Self {
        self.append(ResponseKind::Say(data.into()))
    }

    pub fn problem(self, data: impl Into<Box<str>>) -> Self {
        self.append(ResponseKind::Problem(data.into()))
    }

    pub fn push(&mut self, kind: ResponseKind) {
        self.kind.push(kind);
    }

    pub fn append(mut self, kind: ResponseKind) -> Self {
        self.kind.push(kind);
        self
    }

    pub fn fold_many(iter: impl IntoIterator<Item = Self>) -> Self {
        iter.into_iter()
            .flat_map(|s| s.kind)
            .fold(Self::empty(), |resp, kind| resp.append(kind))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ::serde::Serialize)]
pub enum ResponseKind {
    Say(Box<str>),
    Reply(Box<str>),
    Problem(Box<str>),
}
