use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use crate::utils::structs::{SharedState, WsQuery};

pub async fn ws_handler(
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