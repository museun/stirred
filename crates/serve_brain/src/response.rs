#[derive(serde::Deserialize, serde::Serialize)]
pub struct Generate {
    pub data: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Error {
    pub msg: String,
}
