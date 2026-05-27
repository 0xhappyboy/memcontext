pub(crate) mod local;
pub(crate) mod sqlite;

use crate::types::{DatabaseType, Message};
use async_trait::async_trait;
pub(crate) use local::*;
pub(crate) use sqlite::*;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub sqlite_storage_path: Option<String>,
}

#[async_trait]
pub trait Database: Send + Sync {
    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    async fn get_recent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>>;

    async fn search_keywords(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>>;

    async fn search_semantic(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>>;

    async fn clear_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>>;

    async fn get_message_count(
        &self,
        session_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>>;

    async fn get_all_sessions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    async fn delete_message(&self, id: u64) -> Result<(), Box<dyn std::error::Error>>;

    async fn update_message(
        &self,
        id: u64,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Factory function to create database instance
pub async fn create_database(
    config: DatabaseConfig,
) -> Result<Box<dyn Database + Send + Sync>, Box<dyn std::error::Error>> {
    match config.db_type {
        DatabaseType::SQLite => {
            let path = config
                .sqlite_storage_path
                .unwrap_or_else(|| "./memcontext.db".to_string());
            let db = SQLiteDatabase::new(&path).await?;
            Ok(Box::new(db) as Box<dyn Database + Send + Sync>)
        }
    }
}
