use crate::config::Config;
use crate::llm::{LlmClient, Message};
use crate::logger::{ClientLogger, UserSummary};
use std::sync::Arc;

/// Chat session for a single client
pub struct ChatSession {
    config: Arc<Config>,
    llm: LlmClient,
    logger: ClientLogger,
    messages: Vec<Message>,
    user_summary: UserSummary,
}

impl ChatSession {
    pub fn new(config: Arc<Config>, logger: ClientLogger) -> Self {
        let llm = LlmClient::new(config.clone());
        
        // Initialize logger and load summary
        let _ = logger.init();
        let _ = logger.log_session_start();
        let user_summary = logger.update_session_start().unwrap_or_default();
        
        // Load chat history for context
        let history = logger.load_today_history();
        let mut messages = Vec::new();
        
        for (role, content) in history {
            let role = match role.as_str() {
                "user" => "user",
                "assistant" | "ai" => "assistant",
                _ => continue,
            };
            messages.push(Message {
                role: role.to_string(),
                content,
            });
        }
        
        Self {
            config,
            llm,
            logger,
            messages,
            user_summary,
        }
    }
    
    /// Get personalized system prompt
    fn system_prompt(&self) -> String {
        let mut prompt = self.config.system_prompt.clone();
        
        if let Some(ref name) = self.user_summary.name {
            prompt.push_str(&format!("\n\nThe user's name is {}. Address them by name occasionally.", name));
        }
        
        if self.user_summary.total_sessions > 1 {
            prompt.push_str(&format!(
                "\nThis is session #{} with this user.",
                self.user_summary.total_sessions
            ));
        }
        
        prompt
    }
    
    /// Build messages for LLM including system prompt
    fn build_messages(&self, user_input: &str) -> Vec<Message> {
        let mut msgs = vec![Message {
            role: "system".to_string(),
            content: self.system_prompt(),
        }];
        
        // Add history
        msgs.extend(self.messages.clone());
        
        // Add current message
        msgs.push(Message {
            role: "user".to_string(),
            content: user_input.to_string(),
        });
        
        msgs
    }
    
    /// Process user input and return response
    pub async fn process_input(&mut self, input: &str) -> Result<String, String> {
        let input = input.trim();
        
        if input.is_empty() {
            return Ok(String::new());
        }
        
        // Handle special commands
        if input.starts_with('/') {
            return self.handle_command(input);
        }
        
        // Log user message
        let _ = self.logger.log_message("user", input);
        
        // Add to history
        self.messages.push(Message {
            role: "user".to_string(),
            content: input.to_string(),
        });
        
        // Build messages for LLM
        let llm_messages = self.build_messages(input);
        
        // Remove duplicate (it's in llm_messages)
        self.messages.pop();
        self.messages.push(Message {
            role: "user".to_string(),
            content: input.to_string(),
        });
        
        // Get response from LLM
        let response = self.llm.chat(llm_messages).await?;
        
        // Log and store assistant response
        let _ = self.logger.log_message("assistant", &response);
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
        });
        
        // Keep message history manageable
        while self.messages.len() > 40 {
            self.messages.remove(0);
        }
        
        Ok(response)
    }
    
    /// Handle slash commands
    fn handle_command(&mut self, input: &str) -> Result<String, String> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");
        
        match cmd.as_str() {
            "/name" => {
                if arg.is_empty() {
                    Ok("Usage: /name <your name>".to_string())
                } else {
                    self.user_summary.name = Some(arg.to_string());
                    let _ = self.logger.set_user_name(arg);
                    Ok(format!("Nice to meet you, {}!", arg))
                }
            }
            "/clear" => {
                self.messages.clear();
                Ok("Chat history cleared.".to_string())
            }
            "/help" => {
                Ok("Commands:\n  /name <name> - Set your name\n  /clear - Clear history\n  /help - Show this\n  /quit - Exit".to_string())
            }
            "/quit" | "/exit" => {
                Err("quit".to_string())
            }
            _ => {
                Ok("Unknown command. Type /help for available commands.".to_string())
            }
        }
    }
    
    /// Get welcome message
    pub fn welcome_message(&self) -> String {
        if let Some(ref name) = self.user_summary.name {
            format!("Welcome back, {}! How can I help you today?", name)
        } else {
            "Welcome! Type /name <your name> to introduce yourself, or just start chatting!".to_string()
        }
    }
}
