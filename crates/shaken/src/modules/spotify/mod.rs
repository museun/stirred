use std::sync::Arc;

use anyhow::Context;
use tokio::sync::Mutex;

use crate::{config, queue::Queue, Binding, Config, Request, Response, SharedState};

mod client;

mod data;
use self::{client::SpotifyClient, data::Song};

pub struct Spotify {
    spotify: SpotifyClient,
    queue: Arc<Mutex<Queue<Song>>>,
}

impl Spotify {
    // TODO make this configurable
    const MUSEUN: &'static str = "museun";
    const HISTORY_LIMIT: usize = 10;

    pub async fn create(state: SharedState) -> anyhow::Result<Binding<Self>> {
        const fn spotify_config(config: &Config) -> &config::Spotify {
            &config.spotify
        }

        let crate::config::Spotify {
            client_id,
            client_secret,
        } = &*state.extract(spotify_config).await;

        let spotify = SpotifyClient::new(client_id, client_secret)?;
        let queue = Arc::new(Mutex::new(Queue::with_capacity(Self::HISTORY_LIMIT)));

        let _ = tokio::task::spawn({
            let queue = Arc::clone(&queue);
            let twitch = state.get::<crate::twitch::HelixClient>().await.clone();
            Self::update_loop(queue, twitch, spotify.clone())
        });

        Binding::create(Self { spotify, queue })
            .bind_this(
                "!song",
                "gets the currently playing song from spotify",
                Self::current,
            )?
            .bind_this(
                "!current",
                "gets the currently playing song from spotify",
                Self::current,
            )?
            .bind_this(
                "!previous",
                "gets the previously played song from spotify",
                Self::previous,
            )?
            .bind_this(
                "!recent",
                "lists recently played songs from spotify",
                Self::recent,
            )
    }

    async fn update_loop(
        queue: Arc<Mutex<Queue<Song>>>,
        twitch: crate::twitch::HelixClient,
        spotify: SpotifyClient,
    ) {
        loop {
            if let Ok([_stream]) = twitch.get_streams([Self::MUSEUN]).await.as_deref() {
                if let Some(song) = spotify.try_get_song().await {
                    queue.lock().await.push(song);
                }
            }
            tokio::task::yield_now().await
        }
    }

    async fn current(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_streaming(Self::MUSEUN).await?;

        if let Some(song) = self.queue.lock().await.last() {
            return req.say(song.to_string()).ok();
        }

        if let Some(song) = self.spotify.try_get_song().await {
            let resp = song.to_string();
            self.queue.lock().await.push(song);
            return req.say(resp).ok();
        }
        req.reply("I don't know").ok()
    }

    async fn previous(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_streaming(Self::MUSEUN).await?;

        let queue = self.queue.lock().await;
        let song = queue.last_nth(1).with_context(|| "I don't know")?;
        req.say(song.to_string()).ok()
    }

    async fn recent(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_streaming(Self::MUSEUN).await?;

        let queue = self.queue.lock().await;
        anyhow::ensure!(!queue.is_empty(), "I don't know");

        queue
            .iter()
            .enumerate()
            .take(Self::HISTORY_LIMIT)
            .fold(Response::empty(), |resp, (i, e)| {
                let t;
                let s: &dyn std::fmt::Display = match i {
                    0 => &"current",
                    1 => &"previous",
                    n => {
                        t = format!("previous -{}", n - 1);
                        &t
                    }
                };
                resp.say(format!("{}: {}", s, e,))
            })
            .ok()
    }
}
