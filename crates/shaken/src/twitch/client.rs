use crate::http::Client;

use super::data::{self, Emote, Stream};

#[derive(Clone)]
pub struct HelixClient {
    agent: Client,
    client_id: String,
    bearer_token: String,
    base: Option<String>,
}

impl HelixClient {
    const BASE_URL: &'static str = "https://api.twitch.tv/helix";

    pub fn new(client_id: &str, bearer_token: &str) -> Self {
        Self::new_with_ep(Option::<String>::None, client_id, bearer_token)
    }

    pub fn new_with_ep(ep: impl Into<Option<String>>, client_id: &str, bearer_token: &str) -> Self {
        Self {
            agent: Client::with(|builder| builder.user_agent(crate::http::USER_AGENT)),
            client_id: client_id.to_string(),
            bearer_token: bearer_token.to_string(),
            base: ep.into().map(Into::into),
        }
    }

    pub async fn get_streams<const N: usize>(
        &self,
        names: [&str; N],
    ) -> anyhow::Result<Vec<Stream>> {
        self.get_response("streams", std::iter::repeat("user_login").zip(names))
            .await
            .map(|data| data.data)
    }

    pub async fn get_global_emotes(&self) -> anyhow::Result<(String, Vec<Emote>)> {
        self.get_response("chat/emotes/global", std::iter::empty())
            .await
            .map(|data| (data.template, data.data))
    }

    pub async fn get_emotes_for(
        &self,
        broadcaster_id: &str,
    ) -> anyhow::Result<(String, Vec<Emote>)> {
        self.get_response(
            "chat/emotes/global",
            [("broadcaster_id", broadcaster_id)].into_iter(),
        )
        .await
        .map(|data| (data.template, data.data))
    }

    async fn get_response<'k, 'v, T>(
        &self,
        ep: &str,
        query: impl IntoIterator<Item = (&'k str, &'v str)> + Send,
    ) -> anyhow::Result<data::Data<T>>
    where
        for<'de> T: ::serde::Deserialize<'de> + Send + 'static,
    {
        let url = format!("{}/{}", self.base.as_deref().unwrap_or(Self::BASE_URL), ep);
        let headers = [
            ("client-id", &*self.client_id),
            ("authorization", &*self.bearer_token),
        ];
        self.agent.get(&url, query, headers).await
    }
}
