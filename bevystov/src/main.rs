use bevystov::app::{App, Framebuffer};
use terminal::bevy::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    let buffer = Framebuffer::new(&mut stdout)?;

    let mut app = App::new(stdout, buffer);

    Terminal::enter(|| app.run())
}
