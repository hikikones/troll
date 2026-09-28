use terminal::*;
use widgets2::{Block, KittyDeleteAll, KittyGraphics};

use crate::{
    modals::{Modal, ModalAction},
    pages::Pages,
};

pub struct App {
    pages: Pages,
    modal: Option<Modal>,
    kitty: KittyGraphics,
    is_running: bool,
}

pub enum Action {
    None,
    Render,
    Forward,
    Backward,
    Modal(Modal),
    Clear,
    Quit,
}

impl App {
    pub fn new() -> Self {
        let mut kitty = KittyGraphics::new();

        Self {
            pages: Pages::new(&mut kitty),
            modal: None,
            kitty,
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        // Render default page
        self.render(terminal)?;

        // Run event loop
        while self.is_running {
            let event = Terminal::read()?;
            let action = self.read_event(event, terminal);
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn read_event(&mut self, event: Event, terminal: &mut Terminal) -> Action {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match self.modal {
                Some(modal) => {
                    if let KeyCode::Esc = key.code {
                        Action::Quit
                    } else {
                        self.on_modal_input(key, modal)
                    }
                }
                None => match key.code {
                    KeyCode::Esc => Action::Quit,
                    KeyCode::Tab => Action::Forward,
                    KeyCode::BackTab => Action::Backward,
                    _ => self.on_page_input(key, terminal),
                },
            },
            Event::Resize(_, _) => Action::Render,
            _ => Action::None,
        }
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
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
            Action::Modal(modal) => {
                self.modal = Some(modal);
                self.render(terminal)?;
            }
            Action::Clear => {
                self.kitty.increase_generation();
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

            let (top, body, bottom) = area.split_ends(1, 1);

            self.pages.render_navigation(top, frame);

            self.on_page_render(body.inner(Margin::proportional(1)), frame);

            frame.push_str("TODO BOTTOM");
            frame.render(bottom, TextOptions::span_center_top());

            if let Some(modal) = self.modal {
                self.on_modal_render(area, frame, modal);
            }

            Ok(())
        })
    }

    fn on_page_render(&mut self, area: Rect, frame: &mut Framebuffer) {
        self.pages.render_page(area, frame, &self.kitty);
    }

    fn on_page_input(&mut self, key: KeyEvent, terminal: &mut Terminal) -> Action {
        self.pages.input_page(key, terminal)
    }

    fn on_modal_input(&mut self, key: KeyEvent, modal: Modal) -> Action {
        let action = match modal {
            Modal::Confirm => {
                if let KeyCode::Enter = key.code {
                    self.modal = None;
                    return Action::Render;
                }
                return Action::None;
            }
            Modal::Custom => self.pages.input_modal(key),
        };

        match action {
            ModalAction::None => Action::None,
            ModalAction::Render => Action::Render,
            ModalAction::Confirm => {
                // TODO: refresh page
                self.modal = None;
                Action::Render
            }
            ModalAction::Cancel => {
                self.modal = None;
                Action::Render
            }
        }
    }

    fn on_modal_render(&mut self, area: Rect, frame: &mut Framebuffer, modal: Modal) {
        match modal {
            Modal::Confirm => {
                let area = area.inner(Margin::symmetric(area.cols() / 4, area.rows() / 4));
                let bg = frame.palette().background().slight_offset().as_color();
                Block::fill(bg).render(area, frame);
                Block::rectangle().render(area, frame);
                frame.push_str(" Confirm ");
                frame.render(area, TextOptions::span_center_top());

                frame.print_fmt(SetSgr([Sgr::Fg(Color::Yellow), Sgr::Bold, Sgr::Reverse]));
                frame.push_str("YOLO");
                frame.render(
                    area.inner(Margin::proportional(1)),
                    TextOptions::span_center().with_fill(),
                );
                frame.print_fmt(Sgr::Reset);
            }
            Modal::Custom => {
                self.pages.render_modal(area, frame);
            }
        }
    }
}
