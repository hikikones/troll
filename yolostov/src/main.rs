use terminal::Terminal;
use yolostov::{
    app::App,
    jukebox::{AudioPlayer, Jukebox},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::new()?;

    let jukebox = Jukebox::new(AudioPlayer::new()?);
    let mut app = App::new(jukebox);

    terminal.enter(|term| app.run(term))
}
