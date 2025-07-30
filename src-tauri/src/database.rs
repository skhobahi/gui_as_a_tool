use sqlx::{sqlite::SqlitePool, Row};
use std::path::PathBuf;
use tracing::info;
use crate::models::*;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use in-memory database for simplicity and to avoid file permission issues
        let database_url = "sqlite::memory:";
        
        info!("Initializing in-memory database");
        
        // Create connection pool
        let pool = SqlitePool::connect(&database_url).await?;
        
        let database = Database { pool };
        
        // Initialize database schema
        database.initialize_schema().await?;
        
        Ok(database)
    }
    
    async fn initialize_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create agents table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                connected_at TEXT NOT NULL,
                last_activity TEXT NOT NULL,
                metadata TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create agent_messages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_messages (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                message_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (agent_id) REFERENCES agents (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create human_requests table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS human_requests (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                request_type TEXT NOT NULL,
                message TEXT NOT NULL,
                options TEXT,
                context TEXT,
                timeout_seconds INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                FOREIGN KEY (agent_id) REFERENCES agents (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create human_responses table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS human_responses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id TEXT NOT NULL,
                response TEXT NOT NULL,
                additional_context TEXT,
                responded_by TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (request_id) REFERENCES human_requests (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        info!("Database schema initialized successfully");
        Ok(())
    }
    
    pub async fn save_agent(&self, agent: &Agent) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO agents 
            (id, name, status, connected_at, last_activity, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&agent.id)
        .bind(&agent.name)
        .bind(agent.status.to_string())
        .bind(agent.connected_at.to_rfc3339())
        .bind(agent.last_activity.to_rfc3339())
        .bind(agent.metadata.as_ref().and_then(|m| serde_json::to_string(m).ok()))
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn save_message(&self, message: &AgentMessage) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            INSERT INTO agent_messages 
            (id, agent_id, message_type, payload, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message.id)
        .bind(&message.agent_id)
        .bind(&message.message_type)
        .bind(serde_json::to_string(&message.payload)?)
        .bind(message.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn save_human_request(&self, request: &HumanInputRequest) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO human_requests 
            (id, agent_id, agent_name, request_type, message, options, context, 
             timeout_seconds, timestamp, status, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.id)
        .bind(&request.agent_id)
        .bind(&request.agent_name)
        .bind(serde_json::to_string(&request.request_type)?)
        .bind(&request.message)
        .bind(serde_json::to_string(&request.options)?)
        .bind(request.context.as_ref().and_then(|c| serde_json::to_string(c).ok()))
        .bind(request.timeout_seconds as i64)
        .bind(request.timestamp.to_rfc3339())
        .bind(request.status.to_string())
        .bind(request.priority.to_string())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn save_human_response(&self, response: &HumanResponse) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            INSERT INTO human_responses 
            (request_id, response, additional_context, responded_by, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&response.request_id)
        .bind(&response.response)
        .bind(&response.additional_context)
        .bind(&response.responded_by)
        .bind(response.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_recent_agents(&self, limit: i64) -> Result<Vec<Agent>, Box<dyn std::error::Error>> {
        let rows = sqlx::query(
            "SELECT * FROM agents ORDER BY last_activity DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut agents = Vec::new();
        for row in rows {
            let metadata_str: Option<String> = row.get("metadata");
            let metadata = metadata_str
                .and_then(|s| serde_json::from_str(&s).ok());
            
            agents.push(Agent {
                id: row.get("id"),
                name: row.get("name"),
                status: match row.get::<String, _>("status").as_str() {
                    "connected" => AgentStatus::Connected,
                    "active" => AgentStatus::Active,
                    _ => AgentStatus::Disconnected,
                },
                connected_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("connected_at"))?.with_timezone(&chrono::Utc),
                last_activity: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("last_activity"))?.with_timezone(&chrono::Utc),
                metadata,
            });
        }
        
        Ok(agents)
    }
    
    pub async fn get_recent_messages(&self, limit: i64) -> Result<Vec<AgentMessage>, Box<dyn std::error::Error>> {
        let rows = sqlx::query(
            "SELECT * FROM agent_messages ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut messages = Vec::new();
        for row in rows {
            messages.push(AgentMessage {
                id: row.get("id"),
                agent_id: row.get("agent_id"),
                message_type: row.get("message_type"),
                payload: serde_json::from_str(&row.get::<String, _>("payload"))?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))?.with_timezone(&chrono::Utc),
            });
        }
        
        Ok(messages)
    }
    
    pub async fn get_recent_human_requests(&self, limit: i64) -> Result<Vec<HumanInputRequest>, Box<dyn std::error::Error>> {
        let rows = sqlx::query(
            "SELECT * FROM human_requests ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut requests = Vec::new();
        for row in rows {
            let request_type_str: String = row.get("request_type");
            let request_type = serde_json::from_str(&request_type_str).unwrap_or(RequestType::Input);
            
            let options_str: String = row.get("options");
            let options = serde_json::from_str(&options_str).unwrap_or_default();
            
            let context_str: Option<String> = row.get("context");
            let context = context_str.and_then(|s| serde_json::from_str(&s).ok());
            
            requests.push(HumanInputRequest {
                id: row.get("id"),
                agent_id: row.get("agent_id"),
                agent_name: row.get("agent_name"),
                request_type,
                message: row.get("message"),
                options,
                context,
                timeout_seconds: row.get::<i64, _>("timeout_seconds") as u32,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))?.with_timezone(&chrono::Utc),
                status: match row.get::<String, _>("status").as_str() {
                    "completed" => RequestStatus::Completed,
                    "timeout" => RequestStatus::Timeout,
                    _ => RequestStatus::Pending,
                },
                priority: match row.get::<String, _>("priority").as_str() {
                    "low" => RequestPriority::Low,
                    "high" => RequestPriority::High,
                    "critical" => RequestPriority::Critical,
                    _ => RequestPriority::Medium,
                },
            });
        }
        
        Ok(requests)
    }
    
    pub async fn cleanup_old_data(&self, days: i64) -> Result<(), Box<dyn std::error::Error>> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();
        
        // Clean up old messages
        sqlx::query("DELETE FROM agent_messages WHERE timestamp < ?")
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await?;
        
        // Clean up old requests
        sqlx::query("DELETE FROM human_requests WHERE timestamp < ?")
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await?;
        
        info!("Cleaned up data older than {} days", days);
        Ok(())
    }
}