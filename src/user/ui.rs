use std::{io::{Write, stdout}, sync::{Arc, atomic::{AtomicU16, AtomicUsize, Ordering}}};

use crossterm::{ExecutableCommand, cursor::MoveTo, event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, read}, execute, queue, style::Print, terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size}};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::user::WsStream;

struct UIState {
    lines: Mutex<Vec<String>>,
    scroll: Arc<AtomicUsize>,
    cols: Arc<AtomicU16>,
    rows: Arc<AtomicU16>,
}

impl UIState {
    fn new() -> std::io::Result<Self> {
        let (cols, rows) = size()?;

        Ok(UIState {
            lines: Mutex::new(Vec::new()),
            scroll: Arc::new(0.into()),
            cols: Arc::new(cols.into()),
            rows: Arc::new(rows.into()),
        })
    }

    async fn add_line<S>(&self, line: S)
    where
        S: Into<String>
    {
        let mut lines = self.lines.lock().await;
        lines.push(line.into());
    }

    async fn scroll(&self, up: bool) {
        let lines = self.lines.lock().await;

        let rows = self.rows.load(Ordering::SeqCst);
        let max_scroll = lines.len().saturating_sub((rows - 3) as usize);

        let current = self.scroll.load(Ordering::SeqCst);
        let new = if up {
            current.saturating_add(1).min(max_scroll)
        } else {
            current.saturating_sub(1)
        };

        self.scroll.store(new, Ordering::SeqCst);
    }

    fn resize(&self, new_cols: u16, new_rows: u16) {
        self.cols.store(new_cols, Ordering::SeqCst);
        self.rows.store(new_rows, Ordering::SeqCst);
    }

    async fn render(&self) -> std::io::Result<()> {
        let mut stdout = stdout();

        let rows = self.rows.load(Ordering::SeqCst);
        let scroll = self.scroll.load(Ordering::SeqCst);

        // Clear lines
        for r in 2..rows {
            queue!(
                stdout,
                MoveTo(0, r),
                Clear(ClearType::CurrentLine),
            )?;
        }

        let lines = self.lines.lock().await;
        let start_idx = lines.len().saturating_sub(scroll + (rows - 3) as usize);
        let end_idx = lines.len().saturating_sub(scroll);

        for (idx, line) in
            lines[start_idx..end_idx].iter().rev().enumerate()
        {
            let row = rows - 2 - idx as u16;
            queue!(
                stdout,
                MoveTo(0,row),
                Print(line),
            )?;
        }
        queue!(stdout, MoveTo(0,rows-1))?;
        stdout.flush()?;

        Ok(())
    }
}

pub struct ChatUI {
    state: Arc<UIState>,
    ws_stream: Option<WsStream>,
}

impl ChatUI {
    pub fn new(ws_stream: WsStream) -> std::io::Result<Self> {
        Ok(
            ChatUI {
                state: Arc::new(UIState::new()?),
                ws_stream: Some(ws_stream),
            }
        )
    }

    pub async fn display(&mut self) -> std::io::Result<()> {
        let ws_stream = self.ws_stream.take()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Other,
                "WebSocket already taken"
            ))?;
        let (mut sender, mut receiver) = ws_stream.split();

        let state = self.state.clone();
        let _receive_handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        state.add_line(text).await;
                        if state.render().await.is_err() {
                            eprintln!("Failed to render the current state.");
                            break;
                        }
                    },
                    Message::Close(_) => {
                        print!("\r[Server] Connection closed.");
                        let _ = stdout().flush();
                    }
                    _ => {},
                }
            }
        });

        let mut curr_line = String::new();
        let state_clone = self.state.clone();

        // Channel to communicate events without blocking
        let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

        let _input_hanle = tokio::task::spawn_blocking(move || {
            loop {
                match read() {
                    Ok(event) => {
                        if input_tx.send(event).is_err() { break; }
                    },
                    Err(_) => break,
                }
            }
        });

        let mut stdout = stdout();

        enable_raw_mode()?;
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(EnableMouseCapture)?;

        let rows = state_clone.rows.load(Ordering::SeqCst);
        execute!(
            stdout,
            MoveTo(0,0),
            Print("Loofa vAlpha0.2"),
            MoveTo(0,1),
            Print("Press Ctrl+C to exit."),
            MoveTo(0,rows-1),
        )?;

        loop {
            tokio::select! {
                Some(event) = input_rx.recv() => {
                    match event {
                        Event::Key(key_event) => {
                            match key_event.code {
                                KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                                    let _ = sender.send(Message::Close(None)).await;
                                    break;
                                },
                                KeyCode::Char(ch) => {
                                    write!(stdout, "{ch}")?;
                                    stdout.flush()?;
                                    curr_line.push(ch);
                                },
                                KeyCode::Backspace => {
                                    write!(stdout, "\x08 \x08")?;
                                    stdout.flush()?;
                                    curr_line.pop();
                                },
                                KeyCode::Enter => {
                                    if let Err(e) = sender.send(Message::Text(curr_line.clone())).await {
                                        eprintln!("Failed to send: {e}.");
                                        break;
                                    }
                                    curr_line.clear();
                                },
                                _ => {},
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            match mouse_event.kind {
                                MouseEventKind::ScrollUp => {
                                    state_clone.scroll(true).await;
                                    if state_clone.render().await.is_err() {
                                        eprintln!("Failed to render the current state.");
                                        break;
                                    }
                                },
                                MouseEventKind::ScrollDown => {
                                    state_clone.scroll(false).await;
                                    if state_clone.render().await.is_err() {
                                        eprintln!("Failed to render the current state.");
                                        break;
                                    }
                                },
                                _ => {},
                            }
                        },
                        Event::Resize(new_cols, new_rows) => state_clone.resize(new_cols, new_rows),
                        _ => {},
                    }
                }
            }
        }

        disable_raw_mode()?;
        stdout.execute(LeaveAlternateScreen)?;
        stdout.execute(DisableMouseCapture)?;

        Ok(())
    }
}