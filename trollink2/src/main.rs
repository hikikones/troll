use std::path::PathBuf;

use terminal::Terminal;
use trollink2::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::stdout();
    let dir = std::env::args().nth(1).unwrap_or_default();
    let mut app = App::new(PathBuf::from(dir));
    terminal.enter(|term| {
        app.run(term)?;
        Ok(())
    })
}
