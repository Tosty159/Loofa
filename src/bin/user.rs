use Loofa::user::ui;

fn main() -> std::io::Result<()> {
	ui::update_loop()?;
	Ok(())
}