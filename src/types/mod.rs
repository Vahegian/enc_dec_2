use serde::{Deserialize, Serialize};

pub mod cli;
pub mod config;

pub struct State {
    pub key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub data_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirLS {
    pub name: String,
    pub enc_name: String,
    pub is_dir: bool,
}
