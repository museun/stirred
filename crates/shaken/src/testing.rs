use std::{borrow::Cow, future::Future, path::PathBuf};

use wiremock::{
    matchers::{method, path, query_param},
    MockBuilder, ResponseTemplate,
};

use crate::{irc::Tags, Binding, BoxedFuture, Callable, Request, Response, SharedState, State};

pub fn insta_settings(prefix: &str) -> impl Drop {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_path(snapshots_dir());
    settings.set_snapshot_suffix(prefix);
    settings.bind_to_scope()
}

pub fn inputs_dir() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("snapshots")
        .join("inputs")
}

pub fn snapshots_dir() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("snapshots")
}

pub struct MockServer(wiremock::MockServer, String);

impl MockServer {
    pub fn address(&self) -> &str {
        let Self(.., addr) = self;
        &addr
    }

    pub async fn read_input_data(name: &str) -> String {
        tokio::fs::read_to_string(inputs_dir().join(name))
            .await
            .unwrap()
    }

    pub async fn mock_get(map: fn(MockBuilder) -> MockBuilder, response: &str) -> Self {
        let server = wiremock::MockServer::start().await;
        let mock = map(wiremock::Mock::given(method("GET"))).respond_with(
            ResponseTemplate::new(200)
                .set_body_string(response)
                .append_header("content-type", "application/json"),
        );

        server.register(mock).await;
        let address = server.address().to_string();
        Self(server, format!("http://{address}"))
    }
}

pub struct MockTwitch(MockServer);
impl std::ops::Deref for MockTwitch {
    type Target = MockServer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MockTwitch {
    pub async fn start_mock_get_streams(response: &str) -> Self {
        Self(
            MockServer::mock_get(
                |m| {
                    m.and(path("/streams"))
                        .and(query_param("user_login", "museun"))
                },
                response,
            )
            .await,
        )
    }

    pub async fn start_mock_global_emotes(response: &str) -> Self {
        Self(MockServer::mock_get(|m| m.and(path("/chat/emotes/global")), response).await)
    }
}

pub trait Mock<'a, F, Fut, T>
where
    Self: Sized + Send + Sync + 'static,
    F: Fn(SharedState) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = anyhow::Result<Binding<T>>> + Send + 'a,
    T: Send + Sync + 'a,
{
    fn mock(self) -> BoxedFuture<'a, TestBinding<T>>;
    fn mock_with_state(self, state: State) -> BoxedFuture<'static, TestBinding<T>>;
}

impl<'a, F, Fut, T> Mock<'a, F, Fut, T> for F
where
    F: Fn(SharedState) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<Binding<T>>> + Send + 'a,
    T: Send + Sync + 'static,
{
    fn mock(self) -> BoxedFuture<'a, TestBinding<T>> {
        Box::pin(async { Self::mock_with_state(self, <_>::default()).await })
    }

    fn mock_with_state(self, state: State) -> BoxedFuture<'static, TestBinding<T>> {
        const DEFAULT_CHANNEL: &str = "#test_channel";
        const DEFAULT_SENDER: &str = "#test_user";

        Box::pin(async move {
            let state = SharedState::new(state);
            let binding = (self)(state.clone()).await.expect("valid binding");
            state
                .insert(crate::help::HelpRegistry::create_from(&binding))
                .await;

            TestBinding {
                binding,
                state,
                responses: Vec::new(),
                channel: Cow::Borrowed(DEFAULT_CHANNEL),
                sender: Cow::Borrowed(DEFAULT_SENDER),
                tags: Tags::default(),
            }
        })
    }
}

pub struct TestBinding<T> {
    binding: Binding<T>,
    state: SharedState,
    responses: Vec<Response>,
    channel: Cow<'static, str>,
    sender: Cow<'static, str>,
    tags: Tags,
}

impl<T> TestBinding<T> {
    fn insert_badge(&mut self, key: &str, val: &str) {
        use std::collections::hash_map::Entry::*;
        match self.tags.map.entry(Box::from("badges")) {
            Occupied(mut e) => {
                let mut s = e.get().to_string();
                s.push_str(&format!(",{key}/{val}"));
                *e.get_mut() = s.into_boxed_str();
            }
            Vacant(e) => {
                e.insert(format!("{key}/{val}").into_boxed_str());
            }
        }
    }

    pub fn get_inner(&self) -> &T
    where
        T: Send + Sync + 'static,
    {
        self.binding.get_inner().expect("this to exist")
    }
}

impl<T> TestBinding<T> {
    pub fn with_sender(mut self, sender: &str) -> Self {
        self.sender = sender.to_string().into();
        self
    }

    pub fn with_channel(mut self, channel: &str) -> Self {
        self.channel = channel.to_string().into();
        self
    }

    pub fn with_broadcaster(mut self) -> Self {
        self.insert_badge("broadcaster", "1");
        self
    }

    pub fn with_moderator(mut self) -> Self {
        self.insert_badge("moderator", "1");
        self
    }

    #[track_caller]
    pub async fn send_message(&mut self, data: &str)
    where
        T: Send + Sync + 'static,
    {
        let Self {
            binding,
            state,
            sender,
            channel,
            responses,
            tags,
            ..
        } = self;

        let arc = std::sync::Arc::from;
        let resp = binding
            .call(Request {
                sender: arc(&**sender),
                target: arc(&**channel),
                data: arc(data),
                state: state.clone(),
                tags: std::sync::Arc::new(tags.clone()),
                ..Request::default()
            })
            .await
            .expect("call should succeed");
        responses.push(resp);
    }

    pub fn get_response(&mut self) -> Response {
        let Self { responses, .. } = self;
        Response::fold_many(responses.drain(..))
    }
}
