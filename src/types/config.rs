use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub key: String,
    pub nonce: String,
    pub data_dir: String,
    pub buffer_size: Option<u64>,
}
