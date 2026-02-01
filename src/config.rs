use std::path::PathBuf;

pub struct Config {
    pub port: u16,
    pub api_base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub system_prompt: String,
    pub logs_dir: PathBuf,
    pub host_key_path: Option<PathBuf>,
}
