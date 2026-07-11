use std::{collections::HashMap, sync::Arc};
use axum::extract::ws::Message;
use tokio::sync::{Mutex, broadcast};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub success: bool,
    pub msg: Option<String>
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(serde::Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub struct AppState {
    pub sessions: Mutex<HashMap<String, String>>,
    pub tx: broadcast::Sender<Message>,
    pub connections: Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>,
}

pub type SharedState = Arc<AppState>;