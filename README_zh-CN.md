<h1 align="center">
    memcontext
</h1>
<h4 align="center">
一个可靠的 LLM 上下文管理引擎
</h4>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

## 功能特性

- 三种召回策略：时序召回、关键词匹配、向量语义搜索
- 两种存储后端：本地 JSON 文件和 SQLite（支持向量存储）
- 嵌入式零配置：无需外部依赖，添加到 Cargo.toml 即可使用
- 会话持久化：应用重启后对话历史不丢失

## 快速开始

```rust
use memcontext::{MemContext, MemContextConfig, StorageType, DatabaseType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建使用 SQLite 存储的上下文
    let config = MemContextConfig {
    storage_type: StorageType::DB,
    db_type: Some(DatabaseType::SQLite),
    sqlite_storage_path: Some("./my_memory.db".to_string()),
    ..Default::default()
    };
    let mem = MemContext::new(config).await?;
    let session_id = "user_session_123";
    // 存储对话
    mem.storage_user_chat(session_id.to_string(), "我叫 Alice".to_string()).await?;
    mem.storage_llm_chat(session_id.to_string(), "你好 Alice！".to_string()).await?;
    // 关键词召回
    let result = mem.recall_keywords(session_id, "Alice", 5).await?;
    println!("{}", result.to_context_string());
    // 语义搜索
    let semantic = mem.recall_vec_semantic(session_id, "我叫什么名字", 3).await?;
    println!("{}", semantic.to_context_string());
    Ok(())
}
```

## API 概览

| 方法                  | 说明              |
| --------------------- | ----------------- |
| storage_user_chat()   | 存储用户消息      |
| storage_llm_chat()    | 存储助手回复      |
| recall_time_series()  | 获取最近 N 条消息 |
| recall_keywords()     | 关键词匹配搜索    |
| recall_vec_semantic() | 语义相似度搜索    |
| clear_session()       | 删除会话所有消息  |
| session_size()        | 获取会话消息数量  |

## 配置示例

```rust
let config = MemContextConfig {
storage_type: StorageType::Local, // Local 或 DB
sqlite_storage_path: Some("./data.db".to_string()), // SQLite 存储路径
local_storage_path: Some("./data".to_string()), // 本地 JSON 存储路径
..Default::default()
};
```
