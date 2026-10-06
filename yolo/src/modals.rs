use terminal::*;
use widgets2::Block;

use crate::app::Colors;

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Custom,
}

#[derive(Debug, Clone, Copy)]
pub enum ModalAction {
    None,
    Render,
    Confirm,
    Cancel,
}

pub struct Modals {
    pub current: Option<Modal>,
    pub search: SearchModal,
    pub cursor_state: CursorState,
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            search: SearchModal::new(),
            cursor_state: CursorState::Hide,
        }
    }

    pub const fn is_none(&self) -> bool {
        self.current.is_none()
    }

    pub const fn is_some(&self) -> bool {
        self.current.is_some()
    }
}

pub struct SearchModal;

impl SearchModal {
    const fn new() -> Self {
        Self
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        let area = area.with_size(area.size / 2).center(area);

        let bg = frame.palette().background().slight_offset().as_color();
        frame.fill(area, bg);

        Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
        frame.print_span_with_options(area, " Search ", SpanOptions::center_top());

        frame.print_span_with_options(
            area.inner(Margin::proportional(1)),
            Styled::new(
                "YOLO",
                Style::fg(colors.secondary).with_bold().with_reverse(),
            ),
            SpanOptions::center().with_fill(),
        );
    }

    pub fn input(&mut self, key: Key) -> ModalAction {
        match key.code {
            KeyCode::Enter => ModalAction::Confirm,
            _ => ModalAction::None,
        }
    }
}
