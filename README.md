<h1 align="center">
    memcontext
</h1>
<h4 align="center">
A reliable LLM context management engine.
</h4>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

## Features

- Three recall strategies: Time-series, keyword matching, and semantic vector search
- Two storage backends: Local JSON files and SQLite (with vector support)
- Embedded and zero-config: No external dependencies, just add to your Cargo.toml
- Session persistence: Save conversation history across application restarts

## Quick Start

```rust
use memcontext::{MemContext, MemContextConfig, StorageType, DatabaseType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a memory context with SQLite storage
    let config = MemContextConfig {
        storage_type: StorageType::DB,
        db_type: Some(DatabaseType::SQLite),
        sqlite_storage_path: Some("./my_memory.db".to_string()),
        ..Default::default()
    };

    let mem = MemContext::new(config).await?;
    let session_id = "user_session_123";

    // Store conversation
    mem.storage_user_chat(session_id.to_string(), "My name is Alice".to_string()).await?;
    mem.storage_llm_chat(session_id.to_string(), "Hello Alice!".to_string()).await?;

    // Recall by keyword
    let result = mem.recall_keywords(session_id, "Alice", 5).await?;
    println!("{}", result.to_context_string());

    // Semantic search
    let semantic = mem.recall_vec_semantic(session_id, "what is my name", 3).await?;
    println!("{}", semantic.to_context_string());

    Ok(())
}
```

## API Overview

| Method                | Description                       |
| --------------------- | --------------------------------- |
| storage_user_chat()   | Store user message                |
| storage_llm_chat()    | Store assistant response          |
| recall_time_series()  | Get most recent N messages        |
| recall_keywords()     | Search by keyword matching        |
| recall_vec_semantic() | Search by semantic similarity     |
| clear_session()       | Delete all messages for a session |
| session_size()        | Get message count for a session   |

## Configuration

```rust
let config = MemContextConfig {
storage_type: StorageType::Local, // Local or DB
sqlite_storage_path: Some("./data.db".to_string()), // for SQLite
local_storage_path: Some("./data".to_string()), // for Local JSON
..Default::default()
};
```
