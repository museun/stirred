#[derive(serde::Deserialize, serde::Serialize)]
pub struct Train {
    pub data: String,
}

#[derive(serde::Deserialize)]
pub struct Generate {
    pub min: usize,
    pub max: usize,
    pub query: Option<String>,
}

impl Default for Generate {
    fn default() -> Self {
        Self {
            min: 3,
            max: 5,
            query: None,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Create {
    pub path: String,
    pub depth: Option<usize>,
}
