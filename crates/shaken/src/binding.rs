use std::{future::Future, sync::Arc};

use crate::{
    arguments::{Arguments, ExampleArgs, Match},
    callable::{BoxedCallable, BoxedResponse},
    error::DontCare,
    util::VecExt as _,
    Callable, Either, Request, Response, State,
};

pub async fn create<T, F, Fut>(state: &mut State, f: F) -> anyhow::Result<Binding<T>>
where
    T: Send + 'static,
    F: Fn(&mut State) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<Binding<T>>> + Send + 'static,
{
    let fut = f(state);
    fut.await
}

enum ThisKind<T> {
    Stateful(Arc<T>),
    Anonymous,
}

impl<T> ThisKind<T> {
    fn try_get_stateful(&self) -> anyhow::Result<Arc<T>> {
        match self {
            Self::Stateful(this) => Ok(Arc::clone(this)),
            Self::Anonymous => anyhow::bail!("bind_this requires a `this` receiver"),
        }
    }
}

pub struct Binding<T> {
    this: ThisKind<T>,
    commands: Vec<BoxedCallable>,
    passives: Vec<BoxedCallable>,
}

impl<T> std::fmt::Debug for Binding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Binding").finish()
    }
}

impl<T> Binding<T>
where
    T: Send + Sync,
{
    pub fn create(this: T) -> Self {
        Self {
            this: ThisKind::Stateful(Arc::new(this)),
            commands: Vec::new(),
            passives: Vec::new(),
        }
    }

    pub const fn anonymous() -> Self {
        Self {
            this: ThisKind::Anonymous,
            commands: Vec::new(),
            passives: Vec::new(),
        }
    }

    pub fn get_inner(&self) -> Option<&T> {
        match &self.this {
            ThisKind::Stateful(this) => Some(this),
            ThisKind::Anonymous => None,
        }
    }

    pub fn into_callable(self) -> BoxedCallable
    where
        T: 'static,
    {
        Arc::new(self)
    }

    pub fn bind_this<F, Fut>(
        mut self,
        command: &'static str,
        help: &'static str,
        callable: F,
    ) -> anyhow::Result<Self>
    where
        T: 'static,
        F: Fn(Arc<T>, Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<Response>> + Send + 'static,
    {
        let example_args = ExampleArgs::parse(command).map(Arc::new)?;
        let this = self.this.try_get_stateful()?;

        let func = move |mut req: Request| {
            let example_args = Arc::clone(&example_args);
            let callable = callable.clone();
            let this = Arc::clone(&this);

            Box::pin(async move {
                let args = match example_args.check_req(command, &mut req) {
                    Either::Left(args) => args,
                    Either::Right(resp) => return Ok(resp),
                };
                req = Request { args, ..req };
                callable(this, req).await
            }) as BoxedResponse
        };

        self.commands.push(Arc::new((command, help, func)));
        Ok(self)
    }

    pub fn bind<C>(
        mut self,
        command: &'static str,
        help: &'static str,
        callable: C,
    ) -> anyhow::Result<Self>
    where
        C: Callable<Request, anyhow::Result<Response>> + Clone + 'static,
        C::Out: Send + 'static,
    {
        let example_args = ExampleArgs::parse(command).map(Arc::new)?;
        let func = move |mut req: Request| {
            let example_args = Arc::clone(&example_args);
            let callable = callable.clone();
            Box::pin(async move {
                let args = match example_args.check_req(command, &mut req) {
                    Either::Left(args) => args,
                    Either::Right(resp) => return Ok(resp),
                };
                req = Request { args, ..req };
                (callable).call(req).await
            }) as BoxedResponse
        };

        self.commands.push(Arc::new((command, help, func)));
        Ok(self)
    }

    pub fn listen_this<F, Fut>(mut self, callable: F) -> anyhow::Result<Self>
    where
        T: 'static,
        F: Fn(Arc<T>, Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<Response>> + Send + 'static,
    {
        let this = self.this.try_get_stateful()?;
        let func = move |req: Request| {
            let callable = callable.clone();
            let this = Arc::clone(&this);
            Box::pin(async move { (callable)(this, req).await }) as BoxedResponse
        };

        self.passives.push(Arc::new(func));
        Ok(self)
    }

    pub fn listen<C>(mut self, callable: C) -> Self
    where
        C: Callable<Request, anyhow::Result<Response>> + Clone + 'static,
        C::Out: Send + 'static,
    {
        let func = move |req: Request| {
            let callable = callable.clone();
            Box::pin(async move { (callable).call(req).await }) as BoxedResponse
        };

        self.passives.push(Arc::new(func));
        self
    }
}

impl<T: Send + Sync> Callable<Request, anyhow::Result<Response>> for Binding<T> {
    type Out = BoxedResponse;

    fn call(&self, req: Request) -> Self::Out {
        let (tx, mut rx) = tokio::sync::mpsc::channel(
            self.commands.len() + self.passives.len(), //
        );

        enum Kind {
            Active,
            Passive,
        }
        impl Kind {
            const fn as_str(&self) -> &'static str {
                match self {
                    Self::Active => "active",
                    Self::Passive => "passive",
                }
            }
        }

        for (i, command) in self.commands.iter().map(Arc::clone).enumerate() {
            let req = req.clone();
            let _ = tokio::spawn({
                let tx = tx.clone();
                async move {
                    let res = command.call(req).await;
                    if let Ok(r) = &res {
                        if r.is_empty() {
                            return;
                        }
                    }
                    let _ = tx.send((i, Kind::Active, res)).await;
                    log::trace!("end call for command: {i}");
                }
            });
        }

        for (i, passive) in self.passives.iter().map(Arc::clone).enumerate() {
            let req = req.clone();
            let _ = tokio::spawn({
                let tx = tx.clone();
                async move {
                    let res = passive.call(req).await;
                    if let Ok(r) = &res {
                        if r.is_empty() {
                            return;
                        }
                    }
                    let _ = tx.send((i, Kind::Passive, res)).await;
                    log::trace!("end call for listen: {i}");
                }
            });
        }

        drop(tx);

        Box::pin(async move {
            // TODO build this with the msg-id
            let mut resp = Response::empty();
            while let Some((i, kind, r)) = rx.recv().await {
                match r {
                    Ok(right) => {
                        if !right.is_empty() {
                            log::trace!("got response for {}: {i}", kind.as_str());
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
        self.commands.iter().filter_map(|c| c.usage()).collect()
    }

    fn all_usage_and_help(&self) -> Vec<(&str, &str)> {
        self.commands
            .iter()
            .filter_map(|c| Some((c.usage()?, c.help()?)))
            .collect()
    }
}

impl<A, B, C> Callable<A, B> for (&'static str, &'static str, C)
where
    C: Callable<A, B>,
    A: Send + 'static,
    B: Send + 'static,
{
    type Out = C::Out;

    fn call(&self, req: A) -> Self::Out {
        let (.., callable) = self;
        (callable).call(req)
    }

    fn usage(&self) -> Option<&str> {
        let (usage, ..) = self;
        Some(usage)
    }

    fn help(&self) -> Option<&str> {
        let (_, help, ..) = self;
        Some(help)
    }
}

impl ExampleArgs {
    fn check_req(&self, command: &str, req: &mut Request) -> Either<Arguments, Response> {
        if !req.match_command(command) {
            return Either::Right(Response::empty());
        }

        let head = std::cmp::min(req.command().len() + 1, req.data().len());
        let input = &req.data()[head..];

        let args = match self.extract(input) {
            Match::Required => {
                let data = format!("an argument is required. usage: {command}");
                return Either::Right(req.problem(data));
            }
            Match::NoMatch => {
                let data = format!("command did not match. usage: {command}");
                return Either::Right(req.problem(data));
            }
            Match::Match(map) => Arguments { map },
            Match::Exact => Arguments::default(),
        };

        Either::Left(args)
    }
}
