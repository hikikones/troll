mod app;
mod pages;

use terminal::Terminal;

use crate::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = Terminal::enter_tui()?;

    let mut app = App::new();
    let res = app.run(&mut terminal);

    terminal.leave_tui()?;

    res
}
