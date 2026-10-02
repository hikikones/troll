use terminal::Terminal;
use yolo::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::new()?;
    let mut app = App::new();

    terminal.enter(|term| app.run(term))
}
