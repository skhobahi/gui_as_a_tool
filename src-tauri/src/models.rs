use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Connected,
    Active,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub agent_id: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInputRequest {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub request_type: RequestType,
    pub message: String,
    pub options: Vec<String>,
    pub context: Option<serde_json::Value>,
    pub timeout_seconds: u32,
    pub timestamp: DateTime<Utc>,
    pub status: RequestStatus,
    pub priority: RequestPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestType {
    Input,
    Approval,
    Choice,
    Confirmation,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Completed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanResponse {
    pub request_id: String,
    pub response: String,
    pub additional_context: Option<String>,
    pub responded_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl RequestPriority {
    pub fn from_request_type_and_message(request_type: &RequestType, message: &str) -> Self {
        match request_type {
            RequestType::Approval | RequestType::Confirmation => RequestPriority::High,
            _ => {
                let message_lower = message.to_lowercase();
                if message_lower.contains("critical") || message_lower.contains("urgent") {
                    RequestPriority::Critical
                } else if message_lower.contains("optional") || message_lower.contains("suggestion") {
                    RequestPriority::Low
                } else {
                    RequestPriority::Medium
                }
            }
        }
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Connected => write!(f, "connected"),
            AgentStatus::Active => write!(f, "active"),
            AgentStatus::Disconnected => write!(f, "disconnected"),
        }
    }
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestStatus::Pending => write!(f, "pending"),
            RequestStatus::Completed => write!(f, "completed"),
            RequestStatus::Timeout => write!(f, "timeout"),
        }
    }
}

impl std::fmt::Display for RequestPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestPriority::Low => write!(f, "low"),
            RequestPriority::Medium => write!(f, "medium"),
            RequestPriority::High => write!(f, "high"),
            RequestPriority::Critical => write!(f, "critical"),
        }
    }
}