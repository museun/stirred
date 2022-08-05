use std::fmt::Write as _;

#[derive(serde::Deserialize)]
pub struct Resp {
    pub crates: Vec<Crate>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Crate {
    name: String,
    max_version: String,
    description: Option<String>,
    documentation: Option<String>,
    repository: Option<String>,
    pub(super) exact_match: bool,
}

impl Crate {
    pub fn format(&self) -> (String, String) {
        let Self {
            name, max_version, ..
        } = self;

        let first = match &self.description {
            Some(desc) => format!("{name} = \"{max_version}\" | {desc}"),
            _ => format!("{name} = {max_version}"),
        };

        let mut second = String::new();
        if let Some(docs) = &self.documentation {
            let _ = second.write_fmt(format_args!("docs: {docs}"));
        }
        if let Some(repo) = &self.repository {
            if !second.is_empty() {
                second.push_str(" | ")
            }
            let _ = second.write_fmt(format_args!("repo: {repo}"));
        }

        (first, second)
    }
}
