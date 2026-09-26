use terminal::*;
use widgets2::{KittyDeleteAll, KittyGraphics};

use crate::pages::Pages;

pub struct App {
    pages: Pages,
    kitty: KittyGraphics,
    is_running: bool,
}

pub enum Action {
    None,
    Render,
    Forward,
    Backward,
    Quit,
}

impl App {
    pub fn new() -> Self {
        let mut kitty = KittyGraphics::new();

        Self {
            pages: Pages::new(&mut kitty),
            kitty,
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Render default page
        self.render(terminal)?;

        // Run event loop
        while self.is_running {
            let event = Terminal::read_event()?;
            let action = self.read_event(event, terminal);
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn read_event(&mut self, event: Event, _terminal: &mut Terminal) -> Action {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Action::None;
                }

                match key.code {
                    KeyCode::Esc => Action::Quit,
                    KeyCode::Tab => Action::Forward,
                    KeyCode::BackTab => Action::Backward,
                    _ => self.on_input(key),
                }
            }
            Event::Resize(_, _) => Action::Render,
            _ => Action::None,
        }
    }

    fn apply_action(
        &mut self,
        action: Action,
        terminal: &mut Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::None => {}
            Action::Render => {
                self.render(terminal)?;
            }
            Action::Forward => {
                self.pages.forward(terminal.frame());
                self.render(terminal)?;
            }
            Action::Backward => {
                self.pages.backward(terminal.frame());
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        terminal.render(|frame| {
            let area = frame.area();

            frame.print_fmt(KittyDeleteAll);

            let [top, body, bottom] = area.split_vertical(
                0,
                [
                    Constraint::Fixed(1),
                    Constraint::Percent(100),
                    Constraint::Fixed(1),
                ],
            );

            self.pages.render_navigation(top, frame);

            self.on_render(body.inner(Margin::proportional(1)), frame);

            frame.push_str("TODO BOTTOM");
            frame.render(bottom, TextOptions::span_center());

            Ok(())
        })
    }

    fn on_render(&mut self, area: Rect, frame: &mut Framebuffer) {
        self.pages.on_render(area, frame, &self.kitty);
    }

    fn on_input(&mut self, key: KeyEvent) -> Action {
        self.pages.on_input(key)
    }
}
