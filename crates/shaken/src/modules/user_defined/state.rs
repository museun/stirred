use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Command {
    pub name: String,
    pub body: String,
    pub author: String,
    pub uses: usize,
}

impl Command {
    pub fn new(
        name: impl Into<String>,
        body: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            body: body.into(),
            author: author.into(),
            uses: 0,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct UserDefinedState {
    map: HashMap<String, Command>,
    aliases: Vec<(String, String)>,
}

impl UserDefinedState {
    pub fn insert(&mut self, command: Command) -> bool {
        if self.map.contains_key(&command.name)
            || self.aliases.iter().any(|(c, _)| c == &command.name)
        {
            return false;
        }

        self.map.insert(command.name.clone(), command);
        true
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.aliases.retain(|(k, v)| k != name && v != name);
        self.map.remove(name).is_some()
    }

    pub fn update(&mut self, name: &str, update: impl Fn(&mut Command)) -> bool {
        if let Some(name) = self.find_name(name).map(ToString::to_string) {
            return self.map.get_mut(&name).map(update).is_some();
        }
        false
    }

    pub fn alias(&mut self, from: &str, to: &str) -> bool {
        // TODO disable cyclic aliasing
        if !self.has(from) || self.has(to) {
            return false;
        }

        if self.aliases.iter().any(|(k, v)| (k == from) ^ (v == to)) {
            return true;
        }

        self.aliases.push((from.to_string(), to.to_string()));
        true
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Command> {
        self.map.get(name).or_else(|| {
            for (k, v) in &self.aliases {
                if v == name {
                    return self.map.get(k);
                }
            }
            None
        })
    }

    pub fn find_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if self.map.contains_key(name) {
            return Some(name);
        }
        for (k, v) in &self.aliases {
            if v == name {
                return Some(k);
            }
        }

        None
    }

    pub fn get_all(&self) -> impl Iterator<Item = &Command> {
        self.map.values()
    }

    pub fn has(&self, name: &str) -> bool {
        self.get_by_name(name).is_some()
    }
}

#[cfg(test)]
mod tests;
