use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, read},
    execute,
    queue,
    style::Print,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size},
};
use std::io::{Write, stdout};

pub struct UiState {
    pub lines: Vec<String>,
    pub input_buf: String,
    pub scroll_offset: usize,
    pub cols: u16,
    pub rows: u16,
}

impl UiState {
    fn new(cols: u16, rows: u16) -> Self {
        UiState {
            lines: vec![],
            input_buf: String::new(),
            scroll_offset: 0,
            cols,
            rows,
        }
    }

    fn push_input(&mut self) {
        self.lines.push(self.input_buf.clone());
        self.input_buf.clear();
    }

    fn scroll(&mut self, kind: MouseEventKind) -> std::io::Result<()> {
        let max_scroll = self.lines.len().saturating_sub((self.rows - 3) as usize);
        match kind {
            MouseEventKind::ScrollUp => {
                if self.scroll_offset < max_scroll {
                    self.scroll_offset += 1;
                }
            },
            MouseEventKind::ScrollDown => self.scroll_offset = self.scroll_offset.saturating_sub(1),
            _ => {},
        }
        self.display()?;
        Ok(())
    }

    fn display(&self) -> std::io::Result<()> {
        for r in 2..self.rows {
            queue!(stdout(), MoveTo(0,r), Clear(ClearType::CurrentLine))?;
        }

        for (idx, line) in
            self.lines.iter().rev().skip(self.scroll_offset).take((self.rows - 3) as usize).enumerate()
        {
            let row = self.rows - 2 - idx as u16;
            queue!(stdout(), MoveTo(0,row), Clear(ClearType::CurrentLine), Print(line))?;
        }
        stdout().flush()?;

        execute!(stdout(), MoveTo(0,self.rows-1))?;
        Ok(())
    }
}

fn init_display() -> std::io::Result<()> {
    execute!(
        stdout(),
        MoveTo(0,0),
        Print("Loofa vAlpha0.2"),
        MoveTo(0,1),
        Print("Press Ctrl+C to exit."),
    )?;
    Ok(())
}

pub fn update_loop() -> std::io::Result<()> {
    let mut stdout = stdout();

    let (cols, rows) = size()?;
    let mut state = UiState::new(cols, rows);

    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    init_display()?;

    execute!(stdout, MoveTo(0,rows-1))?;
    loop {
        match read()? {
            Event::Key(key_event) => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char(ch) => {
                        write!(stdout, "{ch}")?;
                        stdout.flush()?;
                        state.input_buf.push(ch);
                    },
                    KeyCode::Backspace => {
                        write!(stdout, "\x08 \x08")?;
                        stdout.flush()?;
                        let _ = state.input_buf.pop();
                    },
                    KeyCode::Enter => {
                        state.push_input();
                        state.display()?;
                    },
                    _ => {},
                }
            },
            Event::Mouse(mouse_event) => state.scroll(mouse_event.kind)?,
            Event::Resize(new_cols, new_rows) => {
                state.cols = new_cols;
                state.rows = new_rows;
            }
            _ => {},
        }
    }

    disable_raw_mode()?;
    stdout.execute(LeaveAlternateScreen)?;
    stdout.execute(DisableMouseCapture)?;
    Ok(())
}