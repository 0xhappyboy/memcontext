use crate::{Database, DatabaseConfig, DatabaseType, Message};
use async_trait::async_trait;

const VECTOR_DIMENSION: usize = 384;

pub struct SQLiteDatabase {
    pool: sqlx::sqlite::SqlitePool,
}

impl SQLiteDatabase {
    pub async fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let connection_string = format!("sqlite:{}?mode=rwc", path);
        let pool = sqlx::SqlitePool::connect(&connection_string).await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                tokens INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_session_id ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_timestamp ON messages(timestamp);
            CREATE TABLE IF NOT EXISTS message_vectors (
                id INTEGER PRIMARY KEY,
                vector BLOB NOT NULL,
                FOREIGN KEY (id) REFERENCES messages(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_vector_id ON message_vectors(id);
            "#,
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    fn generate_vector(content: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; VECTOR_DIMENSION];
        let content_lower = content.to_lowercase();
        for ch in content_lower.chars() {
            let hash = ch as u64;
            let idx = (hash as usize) % VECTOR_DIMENSION;
            vector[idx] += 1.0;
        }
        let chars: Vec<char> = content_lower.chars().collect();
        for i in 0..chars.len().saturating_sub(1) {
            let bigram = format!("{}{}", chars[i], chars[i + 1]);
            let hash = bigram.chars().fold(0u64, |h, c| h.wrapping_add(c as u64));
            let idx = (hash as usize) % VECTOR_DIMENSION;
            vector[idx] += 0.5;
        }
        let norm: f32 = vector.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }
        vector
    }

    fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &v in vector {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }

    fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
        let mut vector = vec![0.0f32; VECTOR_DIMENSION];
        for (i, chunk) in blob.chunks(4).enumerate() {
            if i < VECTOR_DIMENSION && chunk.len() == 4 {
                vector[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
        }
        vector
    }

    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = v1.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = v2.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm1 > 0.0 && norm2 > 0.0 {
            dot / (norm1 * norm2)
        } else {
            0.0
        }
    }
}

#[async_trait]
impl Database for SQLiteDatabase {
    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = self.pool.begin().await?;
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO messages (session_id, role, content) VALUES (?, ?, ?) RETURNING id",
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .fetch_one(&mut *tx)
        .await?;
        let vector = Self::generate_vector(content);
        let blob = Self::vector_to_blob(&vector);
        sqlx::query("INSERT INTO message_vectors (id, vector) VALUES (?, ?)")
            .bind(id)
            .bind(blob)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_recent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<i32>,
            ),
        >(
            "SELECT id, session_id, role, content, timestamp, tokens FROM messages 
         WHERE session_id = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(Message {
                id: Some(row.0 as u64),
                session_id: row.1,
                role: row.2,
                content: row.3,
                timestamp: row.4,
                tokens: row.5.map(|t| t as usize),
            });
        }
        messages.reverse();
        Ok(messages)
    }

    async fn search_keywords(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let keywords: Vec<&str> = query.split_whitespace().collect();
        let mut conditions = Vec::new();
        for _ in 0..keywords.len() {
            conditions.push("content LIKE ?".to_string());
        }
        let sql = format!(
            "SELECT id, session_id, role, content, timestamp, tokens FROM messages 
             WHERE session_id = ? AND ({}) 
             ORDER BY id DESC LIMIT ?",
            conditions.join(" OR ")
        );
        let mut query_builder = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<i32>,
            ),
        >(&sql);
        query_builder = query_builder.bind(session_id);
        for kw in &keywords {
            query_builder = query_builder.bind(format!("%{}%", kw));
        }
        query_builder = query_builder.bind(top_k as i64);
        let rows = query_builder.fetch_all(&self.pool).await?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(Message {
                id: Some(row.0 as u64),
                session_id: row.1,
                role: row.2,
                content: row.3,
                timestamp: row.4,
                tokens: row.5.map(|t| t as usize),
            });
        }
        Ok(messages)
    }

    async fn search_semantic(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let query_vector = Self::generate_vector(query);
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<i32>,
                Vec<u8>,
            ),
        >(
            "SELECT m.id, m.session_id, m.role, m.content, m.timestamp, m.tokens, v.vector 
             FROM messages m 
             JOIN message_vectors v ON m.id = v.id 
             WHERE m.session_id = ?",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        let mut scored: Vec<(f32, Message)> = Vec::new();
        for row in rows {
            let message = Message {
                id: Some(row.0 as u64),
                session_id: row.1,
                role: row.2,
                content: row.3,
                timestamp: row.4,
                tokens: row.5.map(|t| t as usize),
            };
            let vector = Self::blob_to_vector(&row.6);
            let similarity = Self::cosine_similarity(&query_vector, &vector);
            if similarity > 0.3 {
                scored.push((similarity, message));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, msg)| msg).collect())
    }

    async fn clear_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM messages WHERE session_id = ?")
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;
        for id in ids {
            sqlx::query("DELETE FROM message_vectors WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_message_count(
        &self,
        session_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE session_id = ?")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0 as usize)
    }

    async fn get_all_sessions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT session_id FROM messages ORDER BY session_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn delete_message(&self, id: u64) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM message_vectors WHERE id = ?")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_message(
        &self,
        id: u64,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
            .bind(content)
            .bind(id as i64)
            .execute(&mut *tx)
            .await?;
        let vector = Self::generate_vector(content);
        let blob = Self::vector_to_blob(&vector);
        sqlx::query("UPDATE message_vectors SET vector = ? WHERE id = ?")
            .bind(blob)
            .bind(id as i64)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
