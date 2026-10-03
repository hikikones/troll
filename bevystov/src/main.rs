use bevystov::app::{App, Framebuffer, Terminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = Terminal::new();
    let buffer = Framebuffer::new(&mut output)?;

    let mut app = App::new(output, buffer);

    terminal::bevy::Terminal::enter(|| app.run())
}
