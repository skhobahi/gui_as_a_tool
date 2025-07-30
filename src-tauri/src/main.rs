// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use tracing::{info, error};

mod websocket_server;
mod agent_protocol;
mod database;
mod models;

use websocket_server::WebSocketServer;
use database::Database;
use models::*;

pub type AppState = Arc<Mutex<AppStateInner>>;

pub struct AppStateInner {
    pub websocket_server: Option<Arc<WebSocketServer>>,
    pub database: Database,
    pub connected_agents: Vec<Agent>,
    pub human_requests: Vec<HumanInputRequest>,
}

// Tauri commands that can be called from the frontend
#[tauri::command]
async fn get_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
    info!("🔍 get_agents command called");
    let app_state = state.lock().await;
    
    // Try database first
    match app_state.database.get_recent_agents(100).await {
        Ok(agents) => {
            info!("📊 Database returned {} agents", agents.len());
            Ok(agents)
        },
        Err(e) => {
            error!("❌ Database query failed: {}, using in-memory data", e);
            let in_memory_agents = app_state.connected_agents.clone();
            info!("📋 In-memory agents: {}", in_memory_agents.len());
            Ok(in_memory_agents)
        }
    }
}

#[tauri::command]
async fn get_human_requests(state: State<'_, AppState>) -> Result<Vec<HumanInputRequest>, String> {
    info!("🔍 get_human_requests command called");
    let app_state = state.lock().await;
    
    // Try database first
    match app_state.database.get_recent_human_requests(100).await {
        Ok(requests) => {
            info!("📊 Database returned {} requests", requests.len());
            Ok(requests)
        },
        Err(e) => {
            error!("❌ Database query failed: {}, using in-memory data", e);
            let in_memory_requests = app_state.human_requests.clone();
            info!("📋 In-memory requests: {}", in_memory_requests.len());
            Ok(in_memory_requests)
        }
    }
}

#[tauri::command]
async fn send_human_response(
    state: State<'_, AppState>,
    request_id: String,
    response: String,
    additional_context: Option<String>,
) -> Result<(), String> {
    let (agent_id, ws_server) = {
        let mut app_state = state.lock().await;
        
        // Find the request and mark it as completed
        if let Some(request) = app_state.human_requests.iter_mut().find(|r| r.id == request_id) {
            request.status = RequestStatus::Completed;
            let agent_id = request.agent_id.clone();
            let ws_server = app_state.websocket_server.as_ref().cloned();
            (Some(agent_id), ws_server)
        } else {
            return Err("Request not found".to_string());
        }
    };
    
    // Send response through WebSocket if server is running
    if let (Some(agent_id), Some(ws_server)) = (agent_id, ws_server) {
        let response_data = HumanResponse {
            request_id: request_id.clone(),
            response,
            additional_context,
            responded_by: "human".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        if let Err(e) = ws_server.send_response_to_agent(&agent_id, response_data).await {
            error!("Failed to send response to agent: {}", e);
            return Err(format!("Failed to send response: {}", e));
        }
    }
    
    info!("Human response sent for request: {}", request_id);
    Ok(())
}

#[tauri::command]
async fn get_websocket_port(state: State<'_, AppState>) -> Result<u16, String> {
    let app_state = state.lock().await;
    if let Some(ws_server) = &app_state.websocket_server {
        Ok(ws_server.get_port())
    } else {
        Err("WebSocket server not running".to_string())
    }
}

#[tauri::command]
async fn test_connection() -> Result<String, String> {
    Ok("Connection test successful!".to_string())
}

async fn setup_app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    // Initialize database
    let database = Database::new().await?;
    
    // Create initial app state
    let app_state = Arc::new(Mutex::new(AppStateInner {
        websocket_server: None,
        database,
        connected_agents: Vec::new(),
        human_requests: Vec::new(),
    }));
    
    // Start WebSocket server
    let ws_server = WebSocketServer::new(app_state.clone()).await?;
    let port = ws_server.get_port();
    
    // Update app state with WebSocket server
    {
        let mut state = app_state.lock().await;
        state.websocket_server = Some(Arc::new(ws_server));
    }
    
    info!("Agent HUD v5 WebSocket server started on port {}", port);
    
    Ok(app_state)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Agent HUD v5...");
    
    // Setup app state with embedded services
    let app_state = match setup_app_state().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to setup app state: {}", e);
            std::process::exit(1);
        }
    };
    
    // Build and run Tauri app
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_agents,
            get_human_requests,
            send_human_response,
            get_websocket_port,
            test_connection
        ])
        .setup(|app| {
            info!("Agent HUD v5 desktop application started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}