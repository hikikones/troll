mod app;
mod pages;

use terminal::Terminal;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let terminal = Terminal::new()?;
    let mut app = App::new();

    terminal.enter(|term| app.run(term))
}
