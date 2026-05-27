use crate::Message;
use crate::{Database, Role};
use async_trait::async_trait;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

pub(crate) struct LocalDatabase {
    root_path: PathBuf,
    sessions: Arc<RwLock<HashMap<String, Vec<Message>>>>,
}

impl LocalDatabase {
    pub async fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let root_path = PathBuf::from(path);
        if !root_path.exists() {
            fs::create_dir_all(&root_path)?;
        }
        let mut sessions = HashMap::new();
        if root_path.exists() {
            for entry in fs::read_dir(&root_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(messages) = serde_json::from_str::<Vec<Message>>(&content) {
                                sessions.insert(session_id.to_string(), messages);
                            }
                        }
                    }
                }
            }
        }
        Ok(Self {
            root_path,
            sessions: Arc::new(RwLock::new(sessions)),
        })
    }

    async fn save_session(
        &self,
        session_id: &str,
        messages: &[Message],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.root_path.join(format!("{}.json", session_id));
        let content = serde_json::to_string_pretty(messages)?;
        fs::write(file_path, content)?;
        Ok(())
    }
}

#[async_trait]
impl Database for LocalDatabase {
    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let messages = sessions
            .entry(session_id.to_string())
            .or_insert_with(Vec::new);
        let role_enum = match role {
            "user" => Role::User,
            "assistant" => Role::LLM,
            _ => Role::User,
        };
        let mut message = Message::new(session_id.to_string(), role_enum, content.to_string());
        message.id = Some(messages.len() as u64 + 1);
        messages.push(message);
        self.save_session(session_id, messages).await?;
        Ok(())
    }

    async fn get_recent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let sessions = self.sessions.read().await;
        if let Some(messages) = sessions.get(session_id) {
            let start = messages.len().saturating_sub(limit);
            Ok(messages[start..].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    async fn search_keywords(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let mut keywords = Vec::new();
        let chars: Vec<char> = query.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] >= '\u{4E00}' && chars[i] <= '\u{9FFF}' {
                keywords.push(chars[i].to_string());
                i += 1;
            } else if chars[i].is_whitespace() {
                i += 1;
            } else {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] < '\u{4E00}' {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if !word.is_empty() {
                    keywords.push(word);
                }
            }
        }
        let sessions = self.sessions.read().await;
        if let Some(messages) = sessions.get(session_id) {
            let mut scored: Vec<(usize, &Message)> = messages
                .iter()
                .map(|msg| {
                    let score = keywords
                        .iter()
                        .filter(|kw| msg.content.to_lowercase().contains(&kw.to_lowercase()))
                        .count();
                    (score, msg)
                })
                .filter(|(score, _)| *score > 0)
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            Ok(scored
                .into_iter()
                .take(top_k)
                .map(|(_, msg)| msg.clone())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn search_semantic(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        // Local file storage does not support semantic search, this is a fallback mechanism.
        eprintln!(
            "Warning: LocalDatabase does not support semantic search, falling back to keyword search"
        );
        self.search_keywords(session_id, query, top_k).await
    }

    async fn clear_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        let file_path = self.root_path.join(format!("{}.json", session_id));
        let _ = fs::remove_file(file_path);
        Ok(())
    }

    async fn get_message_count(
        &self,
        session_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).map(|v| v.len()).unwrap_or(0))
    }

    async fn get_all_sessions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }

    async fn delete_message(&self, id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        for (session_id, messages) in sessions.iter_mut() {
            messages.retain(|m| m.id != Some(id));
            if !messages.is_empty() {
                self.save_session(session_id, messages).await?;
            }
        }
        Ok(())
    }

    async fn update_message(
        &self,
        id: u64,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        for (session_id, messages) in sessions.iter_mut() {
            if let Some(msg) = messages.iter_mut().find(|m| m.id == Some(id)) {
                msg.content = content.to_string();
                self.save_session(session_id, messages).await?;
                break;
            }
        }
        Ok(())
    }
}
