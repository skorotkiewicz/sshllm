mod config;
mod chat;
mod llm;
mod logger;
mod server;

use anyhow::Result;
use clap::Parser;
use russh::server::Server as _;
use russh::keys::{PrivateKey, Algorithm};
use russh::keys::ssh_key::LineEnding;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use russh::keys::signature::rand_core::OsRng;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::server::SshServer;

/// SSH LLM Chat Server
#[derive(Parser, Debug)]
#[command(name = "sshllm")]
#[command(about = "SSH server for LLM chat with OpenAI-compatible API")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "2222", env = "SSHLLM_PORT")]
    port: u16,

    /// LLM API endpoint
    #[arg(short = 'e', long = "endpoint", default_value = "http://[IP_ADDRESS]:[PORT]/v1", env = "SSHLLM_API_URL")]
    api_url: String,

    /// Model name
    #[arg(short, long, default_value = "default", env = "SSHLLM_MODEL")]
    model: String,

    /// Logs directory
    #[arg(short, long, default_value = "logs", env = "SSHLLM_LOGS_DIR")]
    logs: PathBuf,

    /// Path to SSH host key
    #[arg(short = 'k', long, default_value = "keys/host_ed25519", env = "SSHLLM_HOST_KEY")]
    host_key: PathBuf,

    /// Custom system prompt
    #[arg(short, long, env = "SSHLLM_SYSTEM_PROMPT")]
    system_prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("sshllm=info".parse()?))
        .init();

    let args = Args::parse();

    // Build config
    let config = Arc::new(Config {
        port: args.port,
        api_base_url: args.api_url.clone(),
        model: args.model.clone(),
        api_key: std::env::var("SSHLLM_API_KEY").ok(),
        system_prompt: args.system_prompt.unwrap_or_else(|| "You are a helpful AI assistant. Be concise and friendly.".to_string()),
        logs_dir: args.logs.clone(),
    });

    // Generate or load host key
    let host_key_path = &args.host_key;
    if let Some(parent) = host_key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let host_key = if host_key_path.exists() {
        info!("Loading host key from {}", host_key_path.display());
        let key_data = std::fs::read_to_string(host_key_path)?;
        PrivateKey::from_openssh(key_data.as_bytes())?
    } else {
        info!("Generating new host key at {}", host_key_path.display());
        let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
        let key_data = key.to_openssh(LineEnding::LF)?;
        std::fs::write(host_key_path, key_data.as_bytes())?;
        key
    };

    info!("Starting sshllm server on port {}", config.port);
    info!("LLM endpoint: {}", config.api_base_url);
    info!("Model: {}", config.model);
    info!("Logs directory: {}", config.logs_dir.display());

    // Configure SSH server
    let ssh_config = russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(1),
        keys: vec![host_key],
        ..Default::default()
    };

    let mut server = SshServer {
        config: config.clone(),
        id: 0,
        clients: Arc::new(Mutex::new(HashMap::new())),
    };

    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    server.run_on_address(Arc::new(ssh_config), addr).await?;

    Ok(())
}
