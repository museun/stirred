use std::time::Duration;

use rspotify::model::{Id, TrackId};

#[derive(Debug, Clone)]
pub struct Song {
    pub id: TrackId,
    pub name: String,
    pub artists: String,
    pub duration: Duration,
    pub progress: Duration,
}

impl std::fmt::Display for Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {} - {}", self.id.url(), self.artists, self.name)
    }
}
