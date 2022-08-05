use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use fastrand_ext::IterExt;
use tokio::sync::Mutex;

use crate::{
    error::ErrorExt, http, twitch::data::EmoteMap, util::IterExt as _, Binding, Request, Response,
    SharedState,
};

struct Config {
    max: usize,
    min: usize,
    cooldown: Duration,
    kappa_chance: f32,
    context_chance: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max: 30,
            min: 3,
            cooldown: Duration::from_secs(30),
            kappa_chance: 0.5,
            context_chance: 0.7,
        }
    }
}

#[derive(Default)]
pub struct AnotherViewer {
    client: http::Client,
    config: Config,
    last: Mutex<Option<Instant>>,
}

impl AnotherViewer {
    pub async fn create(_: SharedState) -> anyhow::Result<Binding<Self>> {
        Binding::create(Self::default())
            .bind_this(
                "!speak <context..>",
                "tries to speak like a twitch viewer, with optional context",
                Self::speak,
            )?
            .listen_this(Self::listen)?
            .listen_this(Self::train)
    }

    async fn speak(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        let query = req.args.get("context").ok();
        if let Some(msg) = self.generate(query).await {
            return req.say(msg).ok();
        }
        Response::nothing()
    }

    // TODO pick this from the channel
    const CREATE: &'static str = "http://localhost:50000/museun/create";
    const TRAIN: &'static str = "http://localhost:50000/museun/train";
    const GENERATE: &'static str = "http://localhost:50000/museun/generate";

    async fn train(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        check!(!req.data.starts_with('!'));

        let data = filters::filter(req.data());
        if data.is_empty() {
            return Response::nothing();
        }

        #[derive(serde::Serialize)]
        struct Train {
            data: String,
        }

        #[derive(serde::Serialize)]
        struct Create {
            path: String,
            depth: usize,
        }

        let body = Create {
            path: "db/museun.sdb".to_string(),
            depth: 5,
        };
        // TODO check this error
        let _ = self.client.post_with_body(Self::CREATE, body).await;

        let body = Train { data };
        self.client
            .post_with_body(Self::TRAIN, body)
            .await
            .dont_care()?;

        Response::nothing()
    }

    async fn listen(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        check!(!req.data.starts_with('!'));

        if let Some(ok) = self.try_mention(&req).await {
            return ok;
        }

        if let Some(ok) = self.try_kappa(&req).await {
            return ok;
        }

        let should_reply = {
            let last = self.last.lock().await;
            last.as_ref().map(Instant::elapsed) >= Some(self.config.cooldown)
        };

        check!(should_reply);

        let ctx = self.try_context(&req);
        let resp = self.generate(ctx).await.dont_care()?;
        req.say(resp).ok()
    }

    async fn try_mention(&self, req: &Request) -> Option<anyhow::Result<Response>> {
        if !req
            .data
            .split_ascii_whitespace()
            .any(AnotherViewer::is_the_bot_name)
        {
            return None;
        }

        let ctx = req
            .data
            .split_ascii_whitespace()
            .filter(|&c| !Self::is_the_bot_name(c))
            .choose(&fastrand::Rng::new());

        self.generate(ctx)
            .await
            .dont_care()
            .map(|resp| req.reply(resp))
            .into()
    }

    async fn try_kappa(&self, req: &Request) -> Option<anyhow::Result<Response>> {
        if fastrand::f32() < self.config.kappa_chance {
            return None;
        }

        let mut words: Vec<_> = req.data.split_ascii_whitespace().collect();
        fastrand::shuffle(&mut words);

        let kappa = {
            let map = req.state.get::<EmoteMap>().await;
            words.into_iter().find(|emote| map.has(emote))?
        };

        self.generate(Some(&kappa))
            .await
            .dont_care()
            .map(|resp| req.say(resp))
            .into()
    }

    fn try_context<'a>(&self, req: &'a Request) -> Option<&'a str> {
        if fastrand::f32() < self.config.context_chance {
            return None;
        }

        let mut data: Vec<_> = req
            .data
            .split_ascii_whitespace()
            .filter(|&c| !Self::is_the_bot_name(c))
            .collect();

        fastrand::shuffle(&mut data);
        while !data.is_empty() {
            if let data @ Some(..) = data.pop().filter(|s| s.len() >= 5) {
                return data;
            }
        }

        None
    }

    // TODO time this out incase the server is stuck in a loop
    async fn generate(&self, query: Option<impl ToString + Send>) -> Option<String> {
        #[derive(serde::Serialize)]
        struct Opts {
            min: usize,
            max: usize,
            query: Option<String>,
        }

        let opts = Opts {
            min: self.config.min,
            max: self.config.max,
            query: query.map(|s| s.to_string()),
        };

        let data = self.client.get_with_body(Self::GENERATE, opts).await.ok()?;

        #[derive(serde::Deserialize)]
        struct Generate {
            data: String,
        }
        let Generate { data } = serde_json::from_str(&data).ok()?;

        self.update_last_seen().await;
        Some(Self::filter_response(data))
    }

    async fn update_last_seen(&self) {
        let mut last = self.last.lock().await;
        last.replace(Instant::now());
    }

    fn filter_response(input: String) -> String {
        if !input.contains('@') {
            return input;
        }
        input
            .split_ascii_whitespace()
            .filter(|s| !s.starts_with('@'))
            .join_with(' ')
    }

    fn is_the_bot_name(input: &str) -> bool {
        // TODO make this configurable
        const HEAD: [char; 6] = ['(', '[', '\'', '\"', '#', '@'];
        const TAIL: [char; 9] = ['.', '?', '!', '\"', '\'', ']', ')', ',', ':'];
        let input = input.trim_start_matches(HEAD).trim_end_matches(TAIL);
        matches!(input, "@shaken_bot" | "shaken_bot" | "shaken")
    }
}

mod filters {
    use regex::Regex;

    use crate::util::IterExt;

    pub fn filter(input: &str) -> String {
        [
            remove_name, //
            remove_links,
            remove_garbage,
            filter_commands,
            filter_mentions,
        ]
        .into_iter()
        .fold(input.to_string(), |input, func| func(&*input))
    }

    fn remove_name(input: &str) -> String {
        static PATTERN: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
            Regex::new(
                r#"(?x)
            ['"\(\[@\#]*?        # leading
            (shaken|shaken_bot) # name
            [,.!?:'"\)\]]*?     # trailing
            "#,
            )
            .unwrap()
        });

        PATTERN.replace_all(input, "").to_string()
    }

    fn filter_commands(input: &str) -> String {
        input
            .split_ascii_whitespace()
            .filter(|c| !c.starts_with('!'))
            .join_with(' ')
    }

    fn filter_mentions(input: &str) -> String {
        input
            .split_ascii_whitespace()
            .filter(|c| !c.starts_with('@'))
            .join_with(' ')
    }

    fn remove_links(input: &str) -> String {
        input
            .split_ascii_whitespace()
            .filter(|c| url::Url::parse(c).is_err())
            .join_with(' ')
    }

    fn remove_garbage(input: &str) -> String {
        input
            .split_ascii_whitespace()
            .filter(|c| c.chars().all(|c| c.is_ascii()))
            .join_with(' ')
    }
}
