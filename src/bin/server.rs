use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/login", post(handle_login))
        .route("/ws", get(ws_handler));

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening at https://127.0.0.1:8080");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_login() {}

async fn ws_handler() {}