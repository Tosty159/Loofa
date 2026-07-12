use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, read},
    execute,
    queue,
    style::Print,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use std::{
    io::{Write, stdout},
    sync::{Arc, atomic::{AtomicU16, AtomicUsize, Ordering}}
};
use crate::user::WsStream;

pub struct UIState {
    pub lines: Arc<Mutex<Vec<String>>>,
    pub scroll_offset: Arc<AtomicUsize>,
    pub cols: Arc<AtomicU16>,
    pub rows: Arc<AtomicU16>,
}

impl UIState {
    fn new() -> Self {
        let (cols, rows) = size().unwrap();
        UIState {
            lines: Arc::new(Mutex::new(Vec::new())),
            scroll_offset: Arc::new(0.into()),
            cols: Arc::new(cols.into()),
            rows: Arc::new(rows.into()),
        }
    }

    async fn add_line(&self, line: String) -> std::io::Result<()> {
        let mut lines = self.lines.lock().await;
        lines.push(line);

        drop(lines);
        self.render().await?;
        Ok(())
    }

    async fn render(&self) -> std::io::Result<()> {
        let rows = self.rows.load(Ordering::SeqCst);
        let cols = self.cols.load(Ordering::SeqCst);

        let mut stdout = stdout();

        for r in 2..rows {
            queue!(stdout, MoveTo(0,r), Clear(ClearType::CurrentLine))?;
        }

        let lines = self.lines.lock().await;
        let scroll = self.scroll_offset.load(Ordering::SeqCst);

        let start_idx = lines.len().saturating_sub(scroll + (rows - 3) as usize);
        let end_idx = lines.len().saturating_sub(scroll);

        for (idx, line) in lines[start_idx..end_idx].iter().rev().enumerate() {
            let row = rows - 2 - idx as u16;
            let truncated = if line.len() > cols as usize {
                format!("{}...", &line[..cols as usize -1])
            } else {
                line.clone()
            };
            queue!(stdout, MoveTo(0, row), Print(truncated))?;
        }
        queue!(stdout, MoveTo(0,rows-1))?;
        stdout.flush()?;

        Ok(())
    }

    // Up = true, Down = false
    async fn scroll(&self, up: bool) -> std::io::Result<()> {
        let rows = self.rows.load(Ordering::SeqCst);

        let lines = self.lines.lock().await;
        let max_scroll = lines.len().saturating_sub((rows - 3) as usize);

        let current = self.scroll_offset.load(Ordering::SeqCst);
        let new = if up { // Scrolling up
            current.saturating_add(1).min(max_scroll)
        } else { // Scrolling down
            current.saturating_sub(1)
        };

        if new != current {
            self.scroll_offset.store(new, Ordering::SeqCst);
            self.render().await?;
        }

        Ok(())
    }

    async fn new_size(&self, new_cols: u16, new_rows: u16) {
        self.cols.store(new_cols, Ordering::SeqCst);
        self.rows.store(new_rows, Ordering::SeqCst);
    }
}

pub struct ChatUI {
    pub state: Arc<UIState>,
    pub ws_stream: Option<WsStream>,
}

impl ChatUI {
    // Initializes the UI
    pub fn new(ws_stream: WsStream) -> Self {
        ChatUI {
            state: Arc::new(UIState::new()),
            ws_stream: Some(ws_stream),
        }
    }

    pub fn init_display(&mut self) -> std::io::Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(EnableMouseCapture)?;

        let rows = self.state.rows.load(Ordering::SeqCst);
        execute!(
            stdout(),
            MoveTo(0,0),
            Print("Loofa vAlpha0.2"),
            MoveTo(0,1),
            Print("Press Ctrl+C to exit."),
            MoveTo(0,rows-1)
        )?;
        Ok(())
    }

    pub async fn display(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout();

        let ws_stream = self.ws_stream.take()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Other,
                "WebSocket already taken"
            ))?;
        let (mut sender, mut receiver) = ws_stream.split();

        let state = self.state.clone();
        let _recieve_handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        if state.add_line(text).await.is_err() {
                            eprintln!("Failed to add line.");
                            break;
                        }
                    },
                    Message::Close(_) => {
                        // terminate connection
                        println!("\r[Server] Connection closed.");
                    },
                    _ => {},
                }
            }
        });

        let mut current_line = String::new();
        let state_clone = self.state.clone();
        loop {
            match read()? {
                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => break,
                        KeyCode::Char(ch) => {
                            write!(stdout, "{ch}")?;
                            stdout.flush()?;
                            current_line.push(ch);
                        },
                        KeyCode::Backspace => {
                            write!(stdout, "\x08 \x08")?;
                            stdout.flush()?;
                            current_line.pop();
                        },
                        KeyCode::Enter => {
                            if let Err(e) = sender.send(Message::Text(current_line.clone())).await {
                                state_clone.add_line(format!("Failed to send: {e}")).await?;
                                break;
                            }
                            let rows = self.state.rows.load(Ordering::SeqCst);
                            queue!(stdout, MoveTo(0, rows-1))?;
                            current_line.clear();
                        },
                        _ => {},
                    }
                },
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => state_clone.scroll(true).await?,
                        MouseEventKind::ScrollDown => state_clone.scroll(false).await?,
                        _ => {},
                    }
                },
                Event::Resize(new_cols, new_rows) => {
                    state_clone.new_size(new_cols, new_rows).await;
                },
                _ => {},
            }
        }

        disable_raw_mode()?;
        stdout.execute(LeaveAlternateScreen)?;
        stdout.execute(DisableMouseCapture)?;
        Ok(())
    } 
}