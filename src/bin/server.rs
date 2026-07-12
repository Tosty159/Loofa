use axum::{
    Router, extract::{Query, State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::{IntoResponse, Response}, routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, sync::{Mutex, broadcast}};
use std::{collections::HashMap, sync::Arc};
use Loofa::utils::structs::{AppState, SharedState, WsQuery};
use Loofa::server::handle_login;

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

async fn ws_handler(
    State(state): State<SharedState>,
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>
) -> Response {
    let username = {
        let sessions = state.sessions.lock().await;
        match sessions.get(&params.token) {
            Some(user) => user.clone(),
            None => {
                return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into_response();
            }
        }
    };

    ws.on_upgrade(move |socket| handle_authenticated_socket(state, socket, username))
}

async fn handle_authenticated_socket(
    state: SharedState,
    socket: WebSocket,
    username: String
) {
    println!("User '{}' connected!", username);

    let (mut sender, mut reciever) = socket.split();
    let (tx, _) = tokio::sync::mpsc::unbounded_channel::<Message>();

    {
        let mut connections = state.connections.lock().await;
        connections.insert(username.clone(), tx);
    }

    let mut broadcast_rx = state.tx.subscribe();

    let _send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = reciever.next().await {
        if let Message::Text(text) = msg {
            broadcast_message(&state, &text, &username).await;
        }
    }

    println!("User '{}' disconnected.", username);
    let mut connections = state.connections.lock().await;
    connections.remove(&username);
}

async fn broadcast_message(
    state: &SharedState,
    text: &String,
    sender: &String
) {
    let msg = format!("{}: {}", sender, text);
    let _ = state.tx.send(Message::Text(msg));
}