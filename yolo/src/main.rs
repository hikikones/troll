use terminal::Terminal;
use yolo::app::App;

fn main() -> std::io::Result<()> {
    let terminal = Terminal::new()?;
    let mut app = App::new();

    terminal.enter(|term| app.run(term))
}
