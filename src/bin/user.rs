use loofa::user::{ChatUI, handle_login, prompt_login, ws_handshake};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let server_url = "https://optionally-helping-python.ngrok-free.app";
	let token = loop {
        let (username, password) = prompt_login();
        match handle_login(server_url, username, password).await {
            Ok(Some(token)) => {
                println!("Login successful.");
                break token;
            },
            Ok(None) => {
                println!("Login failed: Invalid credentials. Please try again.");
            },
            Err(e) => {
                eprintln!("Login error: {e}");
                println!("Please try again.");
            }
        }
    };
	
	let ws_stream = match ws_handshake(server_url, &token).await {
		Ok(w) => w,
		Err(e) => {
			eprintln!("WebSocket error: {e}");
			return Err(e);
		}
	};

	let mut ui = ChatUI::new(ws_stream)?;
    ui.display().await?;

	Ok(())
}