use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyModifiers, read},
	execute,
    ExecutableCommand,
	terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, size},
	style::Print,
    queue,
};
use std::io::{Stdout, Write, stdout};

fn show_lines(last_row: u16, lines: &Vec<String>) -> std::io::Result<()> {
	// Header
	execute!(
		stdout(),
		MoveTo(0, 0),
		Print("Loofa vAlpha0.1"),
		MoveTo(0, 1),
		Print("Press Ctrl+C to stop the program."),
	)?;
	stdout().flush()?;

	let mut row = last_row-1;
	let mut i = 1;
	let size = lines.len();
	while row > 1 && i <= size {
		queue!(stdout(), MoveTo(0, row))?;
		let l = lines.get(size - i).unwrap();
		write!(stdout(), "{l}")?;

		i += 1;
		row -= 1;
	}
	stdout().flush()?;
	Ok(())
}

fn update_display(stdout: &mut Stdout, lines: &[String], rows: u16) -> std::io::Result<()> {
	for r in 2..rows-1 {
		queue!(stdout, MoveTo(0, r), Clear(ClearType::CurrentLine))?;
	}

	for (i, line) in lines.iter().rev().take((rows - 3) as usize).enumerate() {
		let row = rows - 2 - i as u16;
		queue!(stdout, MoveTo(0, row), Clear(ClearType::CurrentLine), Print(line))?;
	}
	stdout.flush()?;
	Ok(())
}

fn main() -> std::io::Result<()> {
	let mut lines: Vec<String> = vec![];
	let mut stdout = stdout();

	enable_raw_mode()?;
	stdout.execute(crossterm::terminal::EnterAlternateScreen)?;

	execute!(
		stdout,
		MoveTo(0, 0),
		Print("Loofa vAlpha0.1"),
		MoveTo(0, 1),
		Print("Press Ctrl+C to stop the program."),
	)?;

    let (_, rows) = size()?;
	update_display(&mut stdout, &lines, rows)?;
	show_lines(rows-1, &lines)?;

	let mut curr_line = String::new();

	queue!(stdout, MoveTo(0, rows-1))?;
    stdout.flush()?;
	loop {
		if let Event::Key(key_event) = read()? {
			match key_event.code {
				KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => break,
				KeyCode::Char(c) => {
					write!(stdout, "{c}")?;
					stdout.flush()?;
					curr_line.push(c);
				},
				KeyCode::Backspace => {
					write!(stdout, "\x08 \x08")?;
					stdout.flush()?;
					let _ = curr_line.pop();
				},
				KeyCode::Enter => {
					lines.push(curr_line.clone());
					curr_line.clear();
					update_display(&mut stdout, &lines, rows)?;

					execute!(
						stdout,
						MoveTo(0, rows-1),
						Clear(ClearType::CurrentLine),
					)?;
				},
				_ => {},
			}
		}
	}

    disable_raw_mode()?;
    stdout.execute(crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}
