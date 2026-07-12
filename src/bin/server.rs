use axum::{
    Router, routing::{get, post},
};
use tokio::{net::TcpListener, sync::{Mutex, broadcast}};
use std::{collections::HashMap, sync::Arc};
use Loofa::utils::structs::AppState;
use Loofa::server::{handle_login, ws_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel(256);
    let state = Arc::new(AppState {
        sessions: Mutex::new(HashMap::new()),
        tx,
        connections: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/login", post(handle_login))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening at https://127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}