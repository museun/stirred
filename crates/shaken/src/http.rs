use std::sync::Arc;

pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub async fn get<'k, 'v, T, I>(ep: &str, query: I) -> anyhow::Result<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + 'static,
    I: IntoIterator<Item = (&'k str, &'v str)> + Send,
{
    let req = Client::build(ep, query, std::iter::empty(), |ep| Client::new().0.get(ep));
    Client::call(req, Client::into_json).await
}

pub async fn post<'k, 'v, T, I>(ep: &str, query: I) -> anyhow::Result<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + 'static,
    I: IntoIterator<Item = (&'k str, &'v str)> + Send,
{
    let req = Client::build(ep, query, std::iter::empty(), |ep| Client::new().0.post(ep));
    Client::call(req, Client::into_json).await
}

#[derive(Clone)]
pub struct Client(Arc<ureq::Agent>);

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self(Arc::new(ureq::agent()))
    }

    pub fn with(f: impl Fn(ureq::AgentBuilder) -> ureq::AgentBuilder) -> Self {
        let builder = ureq::AgentBuilder::new();
        Self(Arc::new(f(builder).build()))
    }

    // TODO Request builder
    pub async fn get_string(&self, ep: &str) -> anyhow::Result<String> {
        let req = Self::build(ep, std::iter::empty(), std::iter::empty(), |ep| {
            self.0.get(ep)
        });
        Self::call(req, Self::into_text).await
    }

    // TODO remove this
    // pub async fn get_string_headers<'qk, 'qv, 'hk, 'hv, Q, H>(
    //     &self,
    //     ep: &str,
    //     query: Q,
    //     headers: H,
    // ) -> anyhow::Result<String>
    // where
    //     Q: IntoIterator<Item = (&'qk str, &'qv str)> + Send,
    //     H: IntoIterator<Item = (&'hk str, &'hv str)> + Send,
    // {
    //     let req = Self::build(ep, query, headers, |ep| self.0.get(ep));
    //     Self::call(req, Self::into_text).await
    // }

    // TODO Request builder
    pub async fn get_with_body(
        &self,
        ep: &str,
        body: impl serde::Serialize + Send + 'static,
    ) -> anyhow::Result<String> {
        let req = Self::build(ep, std::iter::empty(), std::iter::empty(), |ep| {
            self.0.get(ep)
        });

        tokio::task::spawn_blocking(move || {
            req.send_json(body)
                .map_err(anyhow::Error::from)?
                .into_string()
                .map_err(anyhow::Error::from)
        })
        .await?
    }

    pub async fn post_with_body(
        &self,
        ep: &str,
        body: impl serde::Serialize + Send + 'static,
    ) -> anyhow::Result<String> {
        let req = Self::build(ep, std::iter::empty(), std::iter::empty(), |ep| {
            self.0.post(ep)
        });

        tokio::task::spawn_blocking(move || {
            req.send_json(body)
                .map_err(anyhow::Error::from)?
                .into_string()
                .map_err(anyhow::Error::from)
        })
        .await?
    }

    pub async fn get<'qk, 'qv, 'hk, 'hv, T, Q, H>(
        &self,
        ep: &str,
        query: Q,
        headers: H,
    ) -> anyhow::Result<T>
    where
        for<'de> T: serde::Deserialize<'de> + Send + 'static,
        Q: IntoIterator<Item = (&'qk str, &'qv str)> + Send,
        H: IntoIterator<Item = (&'hk str, &'hv str)> + Send,
    {
        let req = Self::build(ep, query, headers, |ep| self.0.get(ep));
        Self::call(req, Self::into_json).await
    }

    pub async fn post<'qk, 'qv, 'hk, 'hv, T, Q, H>(
        &self,
        ep: &str,
        query: Q,
        headers: H,
    ) -> anyhow::Result<T>
    where
        for<'de> T: serde::Deserialize<'de> + Send + 'static,
        Q: IntoIterator<Item = (&'qk str, &'qv str)> + Send,
        H: IntoIterator<Item = (&'hk str, &'hv str)> + Send,
    {
        let req = Self::build(ep, query, headers, |ep| self.0.post(ep));
        Self::call(req, Self::into_json).await
    }

    async fn call<T, F>(req: ureq::Request, then: F) -> anyhow::Result<T>
    where
        F: FnOnce(ureq::Response) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(move || then(req.call()?)).await?
    }

    fn into_json<T>(resp: ureq::Response) -> anyhow::Result<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        Ok(resp.into_json()?)
    }

    fn into_text(resp: ureq::Response) -> anyhow::Result<String> {
        Ok(resp.into_string()?)
    }

    fn build<'qk, 'qv, 'hk, 'hv, Q, H>(
        ep: &str,
        query: Q,
        headers: H,
        method: impl Fn(&str) -> ureq::Request,
    ) -> ureq::Request
    where
        Q: IntoIterator<Item = (&'qk str, &'qv str)>,
        H: IntoIterator<Item = (&'hk str, &'hv str)>,
    {
        let (q, s) = (ureq::Request::query, ureq::Request::set);
        let query_map = move |req, (key, val)| q(req, key, val);
        let set_map = move |req, (key, val)| s(req, key, val);
        let query = query.into_iter().fold(method(ep), query_map);
        headers.into_iter().fold(query, set_map)
    }
}
