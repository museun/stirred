use std::sync::Arc;

use anyhow::Context as _;
use rspotify::{
    clients::OAuthClient,
    model::{CurrentlyPlayingType, PlayableItem, TrackId},
    AuthCodeSpotify, Credentials, OAuth,
};
use tokio::sync::Mutex;

use super::data::Song;
use crate::util::IterExt as _;

#[derive(Clone)]
pub struct SpotifyClient {
    client: Arc<AuthCodeSpotify>,
    seen: Arc<Mutex<Option<TrackId>>>,
}

impl SpotifyClient {
    pub fn new(client_id: &str, client_secret: &str) -> anyhow::Result<Self> {
        let credentials = Credentials::new(client_id, client_secret);

        let oauth = OAuth::from_env(rspotify::scopes!(
            "user-read-playback-state",
            "user-read-currently-playing"
        ))
        .with_context(|| "cannot get rspotify oauth pref")?;

        let config = rspotify::Config {
            token_cached: true,
            token_refreshing: true,
            ..rspotify::Config::default()
        };

        let mut auth = AuthCodeSpotify::with_config(credentials, oauth, config);
        // TODO this is synchronous
        let url = auth.get_authorize_url(false)?;
        auth.prompt_for_token(&url)?;

        Ok(Self {
            client: Arc::new(auth),
            seen: <_>::default(),
        })
    }

    pub async fn try_get_song(&self) -> Option<Song> {
        // TODO this is synchronouss

        let song = tokio::task::spawn_blocking({
            let client = self.client.clone();
            move || client.current_playing(None, <Option<Option<_>>>::None)
        })
        .await
        .ok()
        .transpose()
        .ok()?
        .flatten()?;

        if !song.is_playing || !matches!(song.currently_playing_type, CurrentlyPlayingType::Track) {
            return None;
        }

        let track = match song.item? {
            PlayableItem::Track(track) => track,
            _ => return None,
        };

        let id = track.id?;
        {
            let seen = &mut *self.seen.lock().await;
            if seen.as_ref() == Some(&id) {
                return None;
            }
            seen.replace(id.clone());
        }

        let artists = track.artists.iter().map(|a| &*a.name);

        Some(Song {
            id,
            name: track.name,
            artists: artists.join_with(','),
            duration: track.duration,
            progress: song.progress?,
        })
    }
}
