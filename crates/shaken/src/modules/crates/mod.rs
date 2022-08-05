use crate::{http, util::Quote, Binding, Request, Response, SharedState};

mod data;

struct CratesClient(http::Client, Option<String>);
impl CratesClient {
    const BASE_URL: &'static str = "https://crates.io/api/v1/crates";

    fn http() -> http::Client {
        http::Client::with(|c| c.user_agent(http::USER_AGENT))
    }

    async fn get(&self, query: &str) -> anyhow::Result<data::Resp> {
        let Self(client, base_url) = self;
        client
            .get(
                base_url.as_deref().unwrap_or(Self::BASE_URL),
                [("page", "1"), ("per_page", "1"), ("q", query)],
                std::iter::empty(),
            )
            .await
    }
}

#[derive(Default)]
pub struct Crates;

impl Crates {
    pub async fn create(state: SharedState) -> anyhow::Result<Binding<Self>> {
        let client = CratesClient(CratesClient::http(), None);
        state.insert(client).await;

        Binding::anonymous()
            .bind("!crate <name>", "lookup a rust crate", lookup)?
            .bind("!crates <name>", "lookup a rust crate", lookup)
    }
}

async fn lookup(req: Request) -> anyhow::Result<Response> {
    let arg = req.args.get("name").map(Quote::Single)?;

    let resp = req.state.get::<CratesClient>().await.get(&arg).await?;
    anyhow::ensure!(!resp.crates.is_empty(), "I can't find anything for: {arg}");

    if let Some(crate_) = resp.crates.iter().find(|c| c.exact_match) {
        let (head, tail) = crate_.format();
        return req.say(head).say(tail).ok();
    }

    if let [crate_, ..] = &*resp.crates {
        let (head, tail) = crate_.format();
        return req.say(format!("closest match: {head}")).say(tail).ok();
    }

    Response::nothing()
}

#[cfg(test)]
mod tests;
