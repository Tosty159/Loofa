use std::io::{Write, stdin, stdout};

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

pub async fn ws_handshake(
    server_url: &str,
    token: &str
) -> Result<(), Box<dyn std::error::Error>> {
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

    let (mut sender, mut reciever) = ws_stream.split();

    let recieve_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = reciever.next().await {
            match msg {
                Message::Text(text) => {
                    println!("\r[Server] {text}");
                    print!("> ");
                    let _ = stdout().flush();
                },
                Message::Close(_) => {
                    println!("\r[Server]: Connection closed.");
                    break;
                },
                _ => {},
            }
        }
        println!("\n[Disconnected] WebSocket connection closed.");
    });

    let mut input = String::new();
    loop {
        print!("> ");
        let _ = stdout().flush();

        input.clear();
        if stdin().read_line(&mut input).is_err() {
            break;
        }

        if let Err(e) = sender.send(Message::Text(input.clone().into())).await {
            eprintln!("Failed to send message: {e}");
            break;
        }
    }

    let _ = recieve_handle.await;
    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(())
}