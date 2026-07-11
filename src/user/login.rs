use std::io::{stdin, stdout, Write};
use crate::utils::structs::{ErrorResponse, LoginRequest, LoginResponse};
use reqwest::Client;

fn read_line(prompt: &str) -> String {
    let mut s = String::new();
    print!("{prompt}");
    let _ = stdout().flush();
    stdin().read_line(&mut s).expect("Invalid string :<");
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }
    
    s
}

pub fn prompt_login() -> (String, String) {
    let username = read_line("Enter username: ");
    let password = read_line("Enter password: ");

    (username, password)
}

pub async fn handle_login(
    server_url: &str,
    username: String,
    password: String,
) -> Result<Option<String>, String> {
    let client = Client::new();
    let url = format!("{server_url}/login");
    let request = LoginRequest { username, password };

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = response.status();
    if status.is_success() {
        match response.json::<LoginResponse>().await {
            Ok(login_data) => {
                if login_data.success {
                    Ok(Some(login_data.token))
                } else {
                    Err(login_data.msg.unwrap_or_else(|| "Login failed.".to_string()))
                }
            },
            Err(e) => Err(format!("Failed to parse JSON: {e}.")),
        }
    } else {
        match response.json::<ErrorResponse>().await {
            Ok(error_data) => Err(error_data.error),
            Err(_) => Err(format!("Login failed with status: {}", status))
        }
    }
}