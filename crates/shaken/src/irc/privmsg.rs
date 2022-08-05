use std::sync::Arc;

use super::Tags;

#[derive(Debug, Clone)]
pub struct Privmsg {
    pub tags: Tags,
    pub user: Arc<str>,
    pub target: Arc<str>,
    pub data: Arc<str>,
    // TODO actions
}

impl Privmsg {
    pub fn msg_id(&self) -> anyhow::Result<uuid::Uuid> {
        self.tags.get_parsed("id")
    }

    pub fn is_from_broadcaster(&self) -> bool {
        self.badge_iter()
            .any(|(key, val)| key == "broadcaster" && val == "1")
    }

    pub fn is_from_moderator(&self) -> bool {
        self.badge_iter()
            .any(|(key, val)| key == "moderator" && val == "1")
    }

    pub fn badge_iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.tags
            .get("badges")
            .into_iter()
            .flat_map(|s| s.split(','))
            .flat_map(|s| s.split_once('/'))
    }

    pub fn match_command(&self, right: &str) -> bool {
        self.command() == Self::split_command(right)
    }

    pub fn command(&self) -> &str {
        Self::split_command(self.data())
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn split_command(input: &str) -> &str {
        input
            .split_once(' ')
            .map(|(k, _)| k)
            .unwrap_or_else(|| input)
    }
}

impl std::fmt::Display for Privmsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.target, self.user, self.data)
    }
}
