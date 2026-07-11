use axum::{
    Json, extract::State, response::{IntoResponse},
};
use crate::utils::structs::{LoginRequest, LoginResponse, SharedState};
use uuid::Uuid;

pub async fn handle_login(
    State(state): State<SharedState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Json(LoginResponse {
            token: String::new(),
            success: false,
            msg: Some("Username and password are required".to_string()),
        });
    }

    let token = Uuid::new_v4().to_string();
    let mut sessions = state.sessions.lock().await;
    sessions.insert(token.clone(), payload.username.clone());

    Json(LoginResponse {
        token,
        success: true,
        msg: Some(format!("Welcome, {}!", payload.username)),
    })
}