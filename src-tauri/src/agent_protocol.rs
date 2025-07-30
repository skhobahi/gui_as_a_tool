use serde::{Deserialize, Serialize};
use crate::models::*;

/// Protocol handler for agent communication
pub struct AgentProtocol;

impl AgentProtocol {
    pub fn validate_message(message: &str) -> Result<ProtocolMessage, ProtocolError> {
        let parsed: serde_json::Value = serde_json::from_str(message)
            .map_err(|e| ProtocolError::InvalidJson(e.to_string()))?;
        
        let message_type = parsed["type"].as_str()
            .ok_or_else(|| ProtocolError::MissingField("type".to_string()))?;
        
        match message_type {
            "register-agent" => Ok(ProtocolMessage::RegisterAgent),
            "agent-message" => Ok(ProtocolMessage::AgentMessage),
            "human-input-request" => Ok(ProtocolMessage::HumanInputRequest),
            "human-input-response" => Ok(ProtocolMessage::HumanInputResponse),
            _ => Err(ProtocolError::UnknownMessageType(message_type.to_string())),
        }
    }
    
    pub fn create_agent_update_message(agent_message: &AgentMessage) -> Result<String, ProtocolError> {
        let message = serde_json::json!({
            "type": "agent-update",
            "data": agent_message,
            "timestamp": agent_message.timestamp.to_rfc3339()
        });
        
        serde_json::to_string(&message)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    pub fn create_human_request_message(request: &HumanInputRequest) -> Result<String, ProtocolError> {
        let message = serde_json::json!({
            "type": "human-input-request",
            "data": request,
            "timestamp": request.timestamp.to_rfc3339()
        });
        
        serde_json::to_string(&message)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    pub fn create_response_acknowledgment(response: &HumanResponse) -> Result<String, ProtocolError> {
        let message = serde_json::json!({
            "type": "human-input-response",
            "requestId": response.request_id,
            "response": response.response,
            "additionalContext": response.additional_context,
            "timestamp": response.timestamp.to_rfc3339()
        });
        
        serde_json::to_string(&message)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    RegisterAgent,
    AgentMessage,
    HumanInputRequest,
    HumanInputResponse,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Unknown message type: {0}")]
    UnknownMessageType(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}