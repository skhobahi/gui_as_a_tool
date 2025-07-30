use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use chrono::Utc;

use crate::models::*;
use crate::AppState;

pub struct WebSocketServer {
    port: u16,
    connected_clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    app_state: AppState,
}

struct ClientConnection {
    id: String,
    client_type: ClientType,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    agent_info: Option<Agent>,
}

#[derive(Debug, Clone)]
enum ClientType {
    Agent,
    GUI,
}

impl WebSocketServer {
    pub async fn new(app_state: AppState) -> Result<Self, Box<dyn std::error::Error>> {
        // Find available port starting from 8080
        let port = Self::find_available_port().await?;
        
        let server = WebSocketServer {
            port,
            connected_clients: Arc::new(RwLock::new(HashMap::new())),
            app_state,
        };
        
        // Start the server
        server.start().await?;
        
        Ok(server)
    }
    
    pub fn get_port(&self) -> u16 {
        self.port
    }
    
    async fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
        for port in 8080..8200 {
            if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                drop(listener);
                return Ok(port);
            }
        }
        Err("No available ports found".into())
    }
    
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on: {}", addr);
        
        let clients = self.connected_clients.clone();
        let app_state = self.app_state.clone();
        
        tokio::spawn(async move {
            while let Ok((stream, peer_addr)) = listener.accept().await {
                info!("New connection from: {}", peer_addr);
                
                let clients_clone = clients.clone();
                let app_state_clone = app_state.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(stream, peer_addr, clients_clone, app_state_clone).await {
                        error!("Connection error: {}", e);
                    }
                });
            }
        });
        
        Ok(())
    }
    
    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let client_id = Uuid::new_v4().to_string();
        
        info!("WebSocket connection established: {} ({})", client_id, peer_addr);
        
        // Create communication channel
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Handle outgoing messages to client
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    error!("Failed to send message to client: {}", e);
                    break;
                }
            }
        });
        
        // Handle incoming messages from client
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received message from {}: {}", client_id, text);
                    
                    if let Err(e) = Self::process_message(
                        &client_id,
                        &text,
                        &clients,
                        &app_state,
                        tx_clone.clone(),
                    ).await {
                        error!("Error processing message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} disconnected", client_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        // Clean up client connection
        Self::cleanup_client(&client_id, &clients, &app_state).await;
        
        Ok(())
    }
    
    async fn process_message(
        client_id: &str,
        message: &str,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
        sender: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let parsed_message: serde_json::Value = serde_json::from_str(message)?;
        let message_type = parsed_message["type"].as_str().unwrap_or("unknown");
        
        match message_type {
            "register-agent" => {
                Self::handle_agent_registration(client_id, &parsed_message, clients, app_state, sender).await?;
            }
            "register-gui" => {
                Self::handle_gui_registration(client_id, clients, sender).await?;
            }
            "agent-message" => {
                Self::handle_agent_message(client_id, &parsed_message, clients, app_state).await?;
            }
            "human-input-request" => {
                Self::handle_human_input_request(client_id, &parsed_message, clients, app_state).await?;
            }
            "human-input-response" => {
                Self::handle_human_input_response(&parsed_message, clients, app_state).await?;
            }
            "markdown-content" => {
                Self::handle_content_emission(client_id, &parsed_message, clients, "markdown-content").await?;
            }
            "code-content" => {
                Self::handle_content_emission(client_id, &parsed_message, clients, "code-content").await?;
            }
            "image-content" => {
                Self::handle_content_emission(client_id, &parsed_message, clients, "image-content").await?;
            }
            _ => {
                warn!("Unknown message type: {}", message_type);
            }
        }
        
        Ok(())
    }
    
    async fn handle_agent_registration(
        client_id: &str,
        message: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
        sender: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let agent_name = message["name"].as_str().unwrap_or("Unknown Agent");
        let metadata = message["metadata"].clone();
        
        let agent = Agent {
            id: client_id.to_string(),
            name: agent_name.to_string(),
            status: AgentStatus::Connected,
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            metadata: if metadata.is_null() { None } else { Some(metadata) },
        };
        
        // Add to connected clients
        {
            let mut clients_lock = clients.write().await;
            clients_lock.insert(client_id.to_string(), ClientConnection {
                id: client_id.to_string(),
                client_type: ClientType::Agent,
                sender,
                agent_info: Some(agent.clone()),
            });
        }
        
        // Add to app state
        {
            let mut app_state_lock = app_state.lock().await;
            app_state_lock.connected_agents.push(agent.clone());
            
            // Also save to database
            if let Err(e) = app_state_lock.database.save_agent(&agent).await {
                error!("Failed to save agent to database: {}", e);
            }
        }
        
        info!("Agent registered: {} ({})", agent_name, client_id);
        
        // Send acknowledgment
        let ack_message = serde_json::json!({
            "type": "registration-ack",
            "success": true,
            "agentId": client_id,
            "serverTime": Utc::now().to_rfc3339()
        });
        
        if let Ok(ack_text) = serde_json::to_string(&ack_message) {
            let clients_lock = clients.read().await;
            if let Some(client) = clients_lock.get(client_id) {
                let _ = client.sender.send(Message::Text(ack_text));
            }
        }
        
        // Notify GUI clients about new agent
        Self::broadcast_to_guis(clients, "agent-connected", &agent).await;
        
        Ok(())
    }
    
    async fn handle_gui_registration(
        client_id: &str,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        sender: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Add GUI client
        {
            let mut clients_lock = clients.write().await;
            clients_lock.insert(client_id.to_string(), ClientConnection {
                id: client_id.to_string(),
                client_type: ClientType::GUI,
                sender,
                agent_info: None,
            });
        }
        
        info!("GUI client registered: {}", client_id);
        Ok(())
    }
    
    async fn handle_agent_message(
        client_id: &str,
        message: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let agent_message = AgentMessage {
            id: message["id"].as_str().unwrap_or(&Uuid::new_v4().to_string()).to_string(),
            agent_id: client_id.to_string(),
            message_type: message["type"].as_str().unwrap_or("unknown").to_string(),
            payload: message["payload"].clone(),
            timestamp: Utc::now(),
        };
        
        // Update agent last activity
        {
            let mut app_state_lock = app_state.lock().await;
            if let Some(agent) = app_state_lock.connected_agents.iter_mut().find(|a| a.id == client_id) {
                agent.last_activity = Utc::now();
                agent.status = AgentStatus::Active;
            }
        }
        
        // Broadcast to GUI clients
        Self::broadcast_to_guis(clients, "agent-update", &agent_message).await;
        
        Ok(())
    }
    
    async fn handle_human_input_request(
        client_id: &str,
        message: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request_id = message["requestId"].as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Get agent name
        let agent_name = {
            let app_state_lock = app_state.lock().await;
            app_state_lock.connected_agents
                .iter()
                .find(|a| a.id == client_id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Unknown Agent".to_string())
        };
        
        let request_type_str = message["inputType"].as_str().unwrap_or("input");
        let request_type = match request_type_str {
            "approval" => RequestType::Approval,
            "choice" => RequestType::Choice,
            "confirmation" => RequestType::Confirmation,
            "text" => RequestType::Text,
            _ => RequestType::Input,
        };
        
        let request_message = message["message"].as_str().unwrap_or("").to_string();
        let priority = RequestPriority::from_request_type_and_message(&request_type, &request_message);
        
        let human_request = HumanInputRequest {
            id: request_id.clone(),
            agent_id: client_id.to_string(),
            agent_name,
            request_type,
            message: request_message,
            options: message["options"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            context: if message["context"].is_null() { None } else { Some(message["context"].clone()) },
            timeout_seconds: message["timeout"].as_u64().unwrap_or(300) as u32,
            timestamp: Utc::now(),
            status: RequestStatus::Pending,
            priority,
        };
        
        // Add to app state
        {
            let mut app_state_lock = app_state.lock().await;
            app_state_lock.human_requests.push(human_request.clone());
            
            // Also save to database
            if let Err(e) = app_state_lock.database.save_human_request(&human_request).await {
                error!("Failed to save human request to database: {}", e);
            }
        }
        
        info!("Human input request created: {} from agent {}", request_id, client_id);
        
        // Broadcast to GUI clients
        Self::broadcast_to_guis(clients, "human-input-request", &human_request).await;
        
        Ok(())
    }
    
    async fn handle_human_input_response(
        message: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request_id = message["requestId"].as_str().unwrap_or("");
        let response = message["response"].as_str().unwrap_or("");
        
        // Update request status in app state
        let agent_id = {
            let mut app_state_lock = app_state.lock().await;
            if let Some(request) = app_state_lock.human_requests.iter_mut().find(|r| r.id == request_id) {
                request.status = RequestStatus::Completed;
                request.agent_id.clone()
            } else {
                return Err("Request not found".into());
            }
        };
        
        // Send response to agent
        let response_message = serde_json::json!({
            "type": "human-input-response",
            "requestId": request_id,
            "response": response,
            "additionalContext": message["additionalContext"],
            "timestamp": Utc::now().to_rfc3339()
        });
        
        if let Ok(response_text) = serde_json::to_string(&response_message) {
            let clients_lock = clients.read().await;
            if let Some(client) = clients_lock.get(&agent_id) {
                let _ = client.sender.send(Message::Text(response_text));
            }
        }
        
        info!("Human response sent to agent {}: {}", agent_id, response);
        
        Ok(())
    }
    
    async fn handle_content_emission(
        client_id: &str,
        message: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        content_type: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Content emission received: {} from agent {}", content_type, client_id);
        
        // Simply forward the content emission to GUI clients
        let forwarded_message = serde_json::json!({
            "type": content_type,
            "data": message["data"],
            "timestamp": Utc::now().to_rfc3339()
        });
        
        if let Ok(message_text) = serde_json::to_string(&forwarded_message) {
            let clients_lock = clients.read().await;
            for client in clients_lock.values() {
                if matches!(client.client_type, ClientType::GUI) {
                    let _ = client.sender.send(Message::Text(message_text.clone()));
                }
            }
        }
        
        Ok(())
    }
    
    async fn broadcast_to_guis<T: serde::Serialize>(
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        message_type: &str,
        data: &T,
    ) {
        let message = serde_json::json!({
            "type": message_type,
            "data": data,
            "timestamp": Utc::now().to_rfc3339()
        });
        
        if let Ok(message_text) = serde_json::to_string(&message) {
            let clients_lock = clients.read().await;
            for client in clients_lock.values() {
                if matches!(client.client_type, ClientType::GUI) {
                    let _ = client.sender.send(Message::Text(message_text.clone()));
                }
            }
        }
    }
    
    async fn cleanup_client(
        client_id: &str,
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        app_state: &AppState,
    ) {
        // Remove from clients
        let client_info = {
            let mut clients_lock = clients.write().await;
            clients_lock.remove(client_id)
        };
        
        // If it was an agent, remove from app state and notify GUIs
        if let Some(client) = client_info {
            if matches!(client.client_type, ClientType::Agent) {
                {
                    let mut app_state_lock = app_state.lock().await;
                    app_state_lock.connected_agents.retain(|a| a.id != client_id);
                }
                
                // Notify GUI clients
                let disconnect_message = serde_json::json!({
                    "agentId": client_id,
                    "name": client.agent_info.as_ref().map(|a| &a.name).unwrap_or(&"Unknown".to_string())
                });
                
                Self::broadcast_to_guis(clients, "agent-disconnected", &disconnect_message).await;
                
                info!("Agent {} disconnected and cleaned up", client_id);
            }
        }
    }
    
    pub async fn send_response_to_agent(
        &self,
        agent_id: &str,
        response: HumanResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = serde_json::json!({
            "type": "human-input-response",
            "requestId": response.request_id,
            "response": response.response,
            "additionalContext": response.additional_context,
            "timestamp": response.timestamp.to_rfc3339()
        });
        
        if let Ok(message_text) = serde_json::to_string(&message) {
            let clients_lock = self.connected_clients.read().await;
            if let Some(client) = clients_lock.get(agent_id) {
                client.sender.send(Message::Text(message_text))?;
                return Ok(());
            }
        }
        
        Err("Agent not found or failed to serialize message".into())
    }
}