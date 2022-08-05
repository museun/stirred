#[derive(Clone, ::serde::Deserialize)]
pub struct OAuth {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,

    #[serde(default)]
    client_id: String,

    #[serde(skip)]
    bearer_token: String,
}

impl OAuth {
    pub async fn create(client_id: &str, client_secret: &str) -> anyhow::Result<Self> {
        assert!(!client_id.is_empty(), "client_id cannot be empty");
        assert!(!client_secret.is_empty(), "client_secret cannot be empty");

        crate::http::post(
            "https://id.twitch.tv/oauth2/token",
            [
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("grant_type", "client_credentials"),
            ],
        )
        .await
        .map(|this: Self| Self {
            client_id: client_id.to_string(),
            bearer_token: format!("Bearer {}", this.access_token),
            ..this
        })
    }

    pub fn get_client_id(&self) -> &str {
        &self.client_id
    }

    pub fn get_bearer_token(&self) -> &str {
        &self.bearer_token
    }
}
