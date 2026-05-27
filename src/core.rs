use crate::db::{Database, DatabaseConfig};
use crate::types::{DatabaseType, RecallResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for MemContext
#[derive(Debug, Clone)]
pub struct MemContextConfig {
    /// Storage path for session data (for file-based databases like SQLite)
    pub storage_path: Option<String>,
    /// Database type to use
    pub db_type: Option<DatabaseType>,
    /// Additional database connection options
    pub connection_string: Option<String>,
}

impl Default for MemContextConfig {
    fn default() -> Self {
        Self {
            storage_path: Some("./memcontext_data".to_string()),
            db_type: Some(DatabaseType::SQLite),
            connection_string: None,
        }
    }
}

/// Main MemContext structure for managing LLM conversation memory
pub struct MemContext {
    config: MemContextConfig,
    db: Arc<dyn Database + Send + Sync>,
}

impl MemContext {
    pub async fn new(config: MemContextConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db_config = DatabaseConfig {
            db_type: config.db_type.clone().unwrap_or(DatabaseType::SQLite),
            storage_path: config.storage_path.clone(),
            connection_string: config.connection_string.clone(),
        };
        let db = crate::db::create_database(db_config).await?;
        Ok(Self {
            config,
            db: Arc::from(db),
        })
    }

    /// Store user chat message
    pub async fn storage_user_chat(
        &self,
        session_id: String,
        chat: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.db.save_message(&session_id, "user", &chat).await?;
        Ok(())
    }

    /// Store LLM chat message
    pub async fn storage_llm_chat(
        &self,
        session_id: String,
        chat: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.db
            .save_message(&session_id, "assistant", &chat)
            .await?;
        Ok(())
    }

    /// Time-series recall, get most recent N messages
    pub async fn recall_time_series(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<RecallResult, Box<dyn std::error::Error>> {
        let messages = self.db.get_recent_messages(session_id, limit).await?;
        Ok(RecallResult {
            strategy: "time_series".to_string(),
            messages,
            metadata: HashMap::new(),
        })
    }

    /// Keyword-based recall, search messages containing keywords
    pub async fn recall_keywords(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<RecallResult, Box<dyn std::error::Error>> {
        let messages = self.db.search_keywords(session_id, query, top_k).await?;
        Ok(RecallResult {
            strategy: "keywords".to_string(),
            messages,
            metadata: HashMap::new(),
        })
    }

    /// Vector semantic recall, search by semantic similarity
    pub async fn recall_vec_semantic(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<RecallResult, Box<dyn std::error::Error>> {
        let messages = self.db.search_semantic(session_id, query, top_k).await?;
        Ok(RecallResult {
            strategy: "vec_semantic".to_string(),
            messages,
            metadata: HashMap::new(),
        })
    }

    /// Get database reference for advanced operations
    pub fn get_db(&self) -> Arc<dyn Database + Send + Sync> {
        self.db.clone()
    }

    /// Clear all messages for a session
    pub async fn clear_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.db.clear_session(session_id).await?;
        Ok(())
    }

    /// Get session message count
    pub async fn session_size(
        &self,
        session_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        self.db.get_message_count(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memcontext_creation() {
        let config = MemContextConfig::default();
        let memcontext = MemContext::new(config).await;
        assert!(memcontext.is_ok());
    }
}
