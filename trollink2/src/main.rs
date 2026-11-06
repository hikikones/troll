use terminal::Terminal;
use trollink2::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::stdout();
    let mut app = App::new();
    terminal.enter(|term| {
        app.run(term)?;
        Ok(())
    })
}
