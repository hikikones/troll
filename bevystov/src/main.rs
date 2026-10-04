use bevystov::app::App;
use terminal::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::new()?;
    let mut app = App::new();

    terminal.enter(|term| app.run(term))
}
