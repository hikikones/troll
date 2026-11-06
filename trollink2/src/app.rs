use terminal::*;

pub struct App {
    frame: Framebuffer,
    is_running: bool,
}

impl App {
    pub const fn new() -> Self {
        Self {
            frame: Framebuffer::new(),
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        self.frame.query_size()?;
        self.render(terminal)?;

        while self.is_running {
            let Some(event) = Terminal::read()? else {
                continue;
            };

            let action = self.handle_event(event, terminal)?;
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: TerminalEvent,
        _terminal: &mut Terminal,
    ) -> std::io::Result<Action> {
        let action = match event {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Esc => Action::Quit,
                _ => Action::None,
            },
            TerminalEvent::Resize => {
                self.frame.query_size()?;
                Action::Render
            }
        };

        Ok(action)
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
        match action {
            Action::None => {}
            Action::Render => {
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        let mut frame = std::mem::take(&mut self.frame);

        let area = frame.area();
        frame.print_span_with(area, "TODO", SpanOptions::center());

        frame.flush(&mut terminal.lock())?;
        self.frame = frame;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    Render,
    Quit,
}
