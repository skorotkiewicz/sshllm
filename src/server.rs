use crate::chat::ChatSession;
use crate::config::Config;
use crate::logger::ClientLogger;
use russh::keys::{PublicKey, PublicKeyBase64};
use russh::server::{Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, CryptoVec, MethodSet};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Per-client state
pub struct ClientState {
    pub handle: russh::server::Handle,
    pub chat_session: Arc<Mutex<ChatSession>>,
    pub input_buffer: String,
}

/// SSH Server
pub struct SshServer {
    pub config: Arc<Config>,
    pub id: usize,
    pub clients: Arc<Mutex<HashMap<usize, ClientState>>>,
}

impl russh::server::Server for SshServer {
    type Handler = SshHandler;

    fn new_client(&mut self, addr: Option<SocketAddr>) -> SshHandler {
        let id = self.id;
        self.id += 1;
        info!("New client connection from {:?}, assigned id {}", addr, id);
        SshHandler {
            config: self.config.clone(),
            id,
            clients: self.clients.clone(),
            client_ip: addr.map(|a| a.ip().to_string()).unwrap_or_else(|| "127.0.0.1".to_string()),
            identity: None,
        }
    }

    fn handle_session_error(&mut self, error: russh::Error) {
        error!("Session error: {:?}", error);
    }
}

/// Per-connection handler
pub struct SshHandler {
    config: Arc<Config>,
    id: usize,
    clients: Arc<Mutex<HashMap<usize, ClientState>>>,
    client_ip: String,
    identity: Option<String>,
}

impl Handler for SshHandler {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Channel opened for client {} (IP: {})", self.id, self.client_ip);
        
        // Final identity: Use key fingerprint if available, otherwise IP
        let final_identity = self.identity.clone().unwrap_or_else(|| self.client_ip.clone());
        
        let logger = ClientLogger::new(&self.config.logs_dir, final_identity);
        let chat_session = Arc::new(Mutex::new(ChatSession::new(self.config.clone(), logger)));
        
        let state = ClientState {
            handle: session.handle(),
            chat_session,
            input_buffer: String::new(),
        };
        
        self.clients.lock().await.insert(self.id, state);
        drop(channel);
        Ok(true)
    }

    async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
        // partial_success: true tells the client "you are partially logged in, 
        // please provide a key if you have one". This helps identify key-users
        // while still allowing guest access.
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::all()),
            partial_success: true,
        })
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        // Generate a fingerprint from the public key bytes
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.public_key_bytes());
        let hash = hasher.finalize();
        let fingerprint = format!("key_{}", &hex::encode(hash));
        
        info!("Client authenticated with key {}", fingerprint);
        self.identity = Some(fingerprint);
        Ok(Auth::Accept)
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        
        let clients = self.clients.lock().await;
        if let Some(state) = clients.get(&self.id) {
            let welcome = state.chat_session.lock().await.welcome_message();
            let banner = format!(
                "\r\n\x1b[1;36m\
                ╔═══════════════════════════════════════════════════════════════════╗\r\n\
                ║                                                                   ║\r\n\
                ║   ███████╗███████╗██╗  ██╗██╗     ██╗     ███╗   ███╗             ║\r\n\
                ║   ██╔════╝██╔════╝██║  ██║██║     ██║     ████╗ ████║             ║\r\n\
                ║   ███████╗███████╗███████║██║     ██║     ██╔████╔██║             ║\r\n\
                ║   ╚════██║╚════██║██╔══██║██║     ██║     ██║╚██╔╝██║             ║\r\n\
                ║   ███████║███████║██║  ██║███████╗███████╗██║ ╚═╝ ██║             ║\r\n\
                ║   ╚══════╝╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝     ╚═╝             ║\r\n\
                ║                                                                   ║\r\n\
                ║                SSH LLM Chat Server                                ║\r\n\
                ╚═══════════════════════════════════════════════════════════════════╝\x1b[0m\r\n\
                {}\r\n\r\n\x1b[1;32mYou: \x1b[0m",
                welcome
            );
            session.data(channel, CryptoVec::from(banner.as_bytes()))?;
        }
        
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let mut clients = self.clients.lock().await;
        
        if let Some(state) = clients.get_mut(&self.id) {
            for &byte in data {
                match byte {
                    // Enter key
                    b'\r' | b'\n' => {
                        let input = std::mem::take(&mut state.input_buffer);
                        let input_trimmed = input.trim().to_string();
                        
                        // Echo newline immediately
                        session.data(channel, CryptoVec::from("\r\n".as_bytes()))?;

                        if !input_trimmed.is_empty() {
                            let handle = state.handle.clone();
                            let chat_session = state.chat_session.clone();
                            
                            // Send thinking indicator immediately to the client
                            session.data(channel, CryptoVec::from("\x1b[1;36mAI:\x1b[0m (thinking...)\r".as_bytes()))?;
                            
                            // Spawn background task for LLM call so we can return and the packet gets sent
                            tokio::spawn(async move {
                                let mut session_lock = chat_session.lock().await;
                                let result = session_lock.process_input(&input_trimmed).await;
                                drop(session_lock);

                                match result {
                                    Ok(response) => {
                                        let response = response.replace('\n', "\r\n");
                                        let output = format!("\x1b[1;36mAI:\x1b[0m {}\r\n\r\n\x1b[1;32mYou: \x1b[0m", response);
                                        let _ = handle.data(channel, CryptoVec::from(output.as_bytes())).await;
                                    }
                                    Err(e) if e == "quit" => {
                                        let _ = handle.data(channel, CryptoVec::from("\r\nGoodbye!\r\n".as_bytes())).await;
                                        let _ = handle.close(channel).await;
                                    }
                                    Err(e) => {
                                        let output = format!("\x1b[1;31mError: {}\x1b[0m\r\n\r\n\x1b[1;32mYou: \x1b[0m", e);
                                        let _ = handle.data(channel, CryptoVec::from(output.as_bytes())).await;
                                    }
                                }
                            });
                        } else {
                            session.data(channel, CryptoVec::from("\x1b[1;32mYou: \x1b[0m".as_bytes()))?;
                        }
                    }
                    // Backspace
                    127 | 8 => {
                        if !state.input_buffer.is_empty() {
                            state.input_buffer.pop();
                            session.data(channel, CryptoVec::from("\x08 \x08".as_bytes()))?;
                        }
                    }
                    // Ctrl+C
                    3 => {
                        session.data(channel, CryptoVec::from("\r\n^C\r\n".as_bytes()))?;
                        session.close(channel)?;
                        return Ok(());
                    }
                    // Regular printable characters
                    32..=126 => {
                        state.input_buffer.push(byte as char);
                        session.data(channel, CryptoVec::from(std::slice::from_ref(&byte)))?;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Channel {:?} closed for client {}", channel, self.id);
        self.clients.lock().await.remove(&self.id);
        Ok(())
    }
}
