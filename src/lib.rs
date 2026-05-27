pub mod core;
pub(crate) mod db;
pub mod types;

pub use core::*;
pub(crate) use db::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap();
        let config = MemContextConfig {
            storage_type: StorageType::Local,
            local_storage_path: Some(storage_path.to_string()),
            db_type: None,
            lancedb_storage_path: None,
            sqlite_storage_path: None,
        };
        let mem = MemContext::new(config).await.unwrap();
        let session_id = "test_session";
        mem.storage_user_chat(session_id.to_string(), "My name is Zhang San".to_string())
            .await
            .unwrap();
        mem.storage_llm_chat(
            session_id.to_string(),
            "Hello Zhang San, nice to meet you".to_string(),
        )
        .await
        .unwrap();
        mem.storage_user_chat(
            session_id.to_string(),
            "I like Rust programming language".to_string(),
        )
        .await
        .unwrap();
        let time_result = mem.recall_time_series(session_id, 2).await.unwrap();
        assert_eq!(time_result.len(), 2);
        assert!(time_result.to_context_string().contains("Rust"));
        let keyword_result = mem
            .recall_keywords(session_id, "Zhang San", 5)
            .await
            .unwrap();
        assert!(keyword_result.len() >= 1);
        assert!(keyword_result.to_context_string().contains("Zhang San"));
        let semantic_result = mem
            .recall_vec_semantic(session_id, "my name", 5)
            .await
            .unwrap();
        assert!(semantic_result.len() >= 1);
        let size = mem.session_size(session_id).await.unwrap();
        assert_eq!(size, 3);
        mem.clear_session(session_id).await.unwrap();
        let size_after_clear = mem.session_size(session_id).await.unwrap();
        assert_eq!(size_after_clear, 0);
    }

    #[tokio::test]
    async fn test_sqlite_storage() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("test.db");
        let storage_path_str = storage_path.to_str().unwrap();
        let config = MemContextConfig {
            storage_type: StorageType::DB,
            db_type: Some(DatabaseType::SQLite),
            sqlite_storage_path: Some(storage_path_str.to_string()),
            lancedb_storage_path: None,
            local_storage_path: None,
        };
        let mem = MemContext::new(config).await.unwrap();
        let session_id = "test_sqlite_session";
        for i in 1..=25 {
            let content = format!("This is message number {}", i);
            mem.storage_user_chat(session_id.to_string(), content.clone())
                .await
                .unwrap();
            mem.storage_llm_chat(session_id.to_string(), format!("Response to: {}", content))
                .await
                .unwrap();
        }
        mem.storage_user_chat(session_id.to_string(), "My name is Li Si".to_string())
            .await
            .unwrap();
        mem.storage_llm_chat(session_id.to_string(), "Hello Li Si".to_string())
            .await
            .unwrap();
        mem.storage_user_chat(session_id.to_string(), "I like eating apples".to_string())
            .await
            .unwrap();
        mem.storage_user_chat(session_id.to_string(), "I work in Shanghai".to_string())
            .await
            .unwrap();
        let total_messages = 25 * 2 + 4;
        let time_result = mem.recall_time_series(session_id, 2).await.unwrap();
        assert_eq!(time_result.len(), 2);
        let time_context = time_result.to_context_string();
        println!("Time-series recall content: {:?}", time_context);
        let has_shanghai = time_context.contains("Shanghai");
        let has_apple = time_context.contains("apples");
        if !has_shanghai && !has_apple {
            println!("All messages:");
            let all_messages = mem.recall_time_series(session_id, 200).await.unwrap();
            for (i, msg) in all_messages.messages.iter().enumerate() {
                println!("  {}: {:?}", i, msg.content);
            }
        }
        assert!(
            has_shanghai || has_apple,
            "Time-series recall should contain 'Shanghai' or 'apples', actual: {}",
            time_context
        );
        let keyword_result = mem
            .recall_keywords(session_id, "Shanghai", 5)
            .await
            .unwrap();
        assert!(keyword_result.len() >= 1);
        assert!(keyword_result.to_context_string().contains("Shanghai"));
        println!(
            "Keyword recall result:\n{}",
            keyword_result.to_context_string()
        );
        let semantic_result = mem
            .recall_vec_semantic(session_id, "my workplace", 3)
            .await
            .unwrap();
        println!("Semantic recall result count: {}", semantic_result.len());
        for msg in &semantic_result.messages {
            println!("  - {}", msg.content);
        }
        assert!(semantic_result.len() >= 1);
        let size = mem.session_size(session_id).await.unwrap();
        assert_eq!(size, total_messages);
        println!("Session size: {}", size);
        mem.clear_session(session_id).await.unwrap();
        let size_after_clear = mem.session_size(session_id).await.unwrap();
        assert_eq!(size_after_clear, 0);
        println!("SQLite storage test passed!");
    }
}
