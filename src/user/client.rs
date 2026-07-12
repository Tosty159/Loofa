use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn ws_handshake(
    server_url: &str,
    token: &str
) -> Result<WsStream, Box<dyn std::error::Error>> {
    let ws_url = if server_url.starts_with("http://") {
        server_url.replace("http://", "ws://")
    } else if server_url.starts_with("https://") {
        server_url.replace("https://", "wss://")
    } else {
        server_url.to_string()
    };

    let url = format!("{ws_url}/ws?token={token}");
    println!("Connecting to: {url}");

    let (ws_stream, response) = connect_async(&url).await?;
    println!("Websocket handshake status: {}", response.status());

    if response.status() != 101 {
        return Err(format!("Websocket connection failed : {}", response.status()).into());
    }
    
    Ok(ws_stream)
}