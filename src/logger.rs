use chrono::{Local, Utc};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct UserSummary {
    pub name: Option<String>,
    pub total_sessions: u32,
}

pub struct ClientLogger {
    base_dir: PathBuf,
}

impl ClientLogger {
    pub fn new(logs_dir: &PathBuf, client_ip: IpAddr) -> Self {
        let base_dir = logs_dir.join(client_ip.to_string());
        Self { base_dir }
    }

    pub fn init(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn summary_path(&self) -> PathBuf {
        self.base_dir.join("summary.txt")
    }

    fn chat_log_path(&self) -> PathBuf {
        let date = Local::now().format("%Y-%m-%d").to_string();
        self.base_dir.join(format!("chat_{}.log", date))
    }

    pub fn update_session_start(&self) -> std::io::Result<UserSummary> {
        let path = self.summary_path();
        let mut summary = UserSummary::default();

        // Read existing summary
        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim();
                    match key.as_str() {
                        "name" => summary.name = Some(value.to_string()),
                        "total_sessions" => {
                            summary.total_sessions = value.parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Increment session count
        summary.total_sessions += 1;

        // Write updated summary
        self.write_summary(&summary)?;

        Ok(summary)
    }

    fn write_summary(&self, summary: &UserSummary) -> std::io::Result<()> {
        let path = self.summary_path();
        let mut file = File::create(&path)?;
        
        if let Some(ref name) = summary.name {
            writeln!(file, "name: {}", name)?;
        }
        writeln!(file, "total_sessions: {}", summary.total_sessions)?;
        writeln!(file, "last_seen: {}", Utc::now().to_rfc3339())?;
        
        Ok(())
    }

    pub fn set_user_name(&self, name: &str) -> std::io::Result<()> {
        let path = self.summary_path();
        let mut summary = UserSummary::default();
        
        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim();
                    if key == "total_sessions" {
                        summary.total_sessions = value.parse().unwrap_or(0);
                    }
                }
            }
        }
        
        summary.name = Some(name.to_string());
        self.write_summary(&summary)
    }

    pub fn log_message(&self, role: &str, content: &str) -> std::io::Result<()> {
        let path = self.chat_log_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        let timestamp = Local::now().format("%H:%M:%S");
        writeln!(file, "[{}] {}: {}", timestamp, role, content)?;
        
        Ok(())
    }

    pub fn load_today_history(&self) -> Vec<(String, String)> {
        let path = self.chat_log_path();
        let mut history = Vec::new();
        
        if let Ok(file) = File::open(&path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                // Parse format: [HH:MM:SS] role: content
                if let Some(rest) = line.strip_prefix('[') {
                    if let Some(idx) = rest.find(']') {
                        let after_time = &rest[idx + 1..].trim();
                        if let Some((role, content)) = after_time.split_once(':') {
                            history.push((role.trim().to_string(), content.trim().to_string()));
                        }
                    }
                }
            }
        }
        
        // Limit history
        if history.len() > 20 {
            history = history.split_off(history.len() - 20);
        }
        
        history
    }
}
