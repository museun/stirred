use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

macro_rules! make_key {
    (@one $key:ident) => {
        const $key: &str = stringify!($key);
    };
    ($($key:ident)*) => {
        $(make_key!(@one $key);)*
    }
}

make_key! {
    // twitch irc
    SHAKEN_TWITCH_IRC_ADDRESS
    SHAKEN_TWITCH_NAME
    SHAKEN_TWITCH_OAUTH_TOKEN
    SHAKEN_TWITCH_CHANNEL
    // twitch api
    SHAKEN_TWITCH_CLIENT_ID
    SHAKEN_TWITCH_CLIENT_SECRET
    // spotify api
    SHAKEN_SPOTIFY_CLIENT_ID
    SHAKEN_SPOTIFY_CLIENT_SECRET
}

#[derive(Debug)]
pub struct Config {
    pub irc: Irc,
    pub twitch: Twitch,
    pub spotify: Spotify,
}

impl Config {
    pub fn load_from_env() -> anyhow::Result<Self> {
        Ok(Self {
            irc: Irc {
                addr: get_var_or(SHAKEN_TWITCH_IRC_ADDRESS, || crate::irc::TWITCH_NO_TLS)?,
                name: get_var_or(SHAKEN_TWITCH_NAME, || "shaken_bot")?,
                pass: get_var(SHAKEN_TWITCH_OAUTH_TOKEN).map(Secret)?,
                channel: get_var_or(SHAKEN_TWITCH_CHANNEL, || "#museun")?,
            },
            twitch: Twitch {
                client_id: get_var(SHAKEN_TWITCH_CLIENT_ID)?,
                client_secret: get_var(SHAKEN_TWITCH_CLIENT_SECRET).map(Secret)?,
            },
            spotify: Spotify {
                client_id: get_var(SHAKEN_SPOTIFY_CLIENT_ID)?,
                client_secret: get_var(SHAKEN_SPOTIFY_CLIENT_SECRET).map(Secret)?,
            },
        })
    }
}

#[derive(Debug)]
pub struct Irc {
    pub addr: String,
    pub name: String,
    pub pass: Secret<String>,
    pub channel: String,
}

#[derive(Debug)]
pub struct Twitch {
    pub client_id: String,
    pub client_secret: Secret<String>,
}

pub struct Spotify {
    pub client_id: String,
    pub client_secret: Secret<String>,
}

impl Debug for Spotify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spotify")
            .field("client_id", &self.client_id)
            .field("client_secret", &self.client_secret.redact())
            .finish()
    }
}

fn get_var(key: &str) -> anyhow::Result<String> {
    anyhow::Context::with_context(std::env::var(key), || {
        anyhow::anyhow!("env var `{key}` must be set")
    })
}

fn get_var_or<T: ToString>(key: &str, def: fn() -> T) -> anyhow::Result<String> {
    get_var(key).or_else(|_e| Ok(def().to_string()))
}

trait Redacted {
    fn redact(&self) -> String;
}

impl Redacted for str {
    fn redact(&self) -> String {
        self.chars().map(|_| 'x').collect()
    }
}

impl Redacted for String {
    fn redact(&self) -> String {
        self.as_str().redact()
    }
}

pub struct Secret<T>(T);

impl<T: Deref<Target = str>> Deref for Secret<T> {
    type Target = T::Target;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Debug + Redacted> Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.redact())
    }
}

impl<T: Display + Redacted> Display for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.redact())
    }
}
