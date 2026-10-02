use terminal::*;
use widgets2::Block;

use crate::app::Colors;

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Logs,
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
    pub cursor_state: CursorState,
    pub search: SearchModal,
    pub logs: LogsModal,
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            cursor_state: CursorState::Hide,
            search: SearchModal::new(),
            logs: LogsModal::new(),
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
        Block::fill(Style::bg(bg)).render(area, frame);
        Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, frame);

        frame.push_str(" Search ");
        frame.render(area, TextOptions::span_center_top());

        frame.print_fmt(Sgrs([Sgr::Fg(colors.secondary), Sgr::Bold, Sgr::Reverse]));
        frame.push_str("YOLO");
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::span_center().with_fill(),
        );
        frame.print_fmt(Sgr::Reset);
    }

    pub fn input(&mut self, key: Key) -> ModalAction {
        match key.code {
            KeyCode::Enter => ModalAction::Confirm,
            _ => ModalAction::None,
        }
    }
}

pub struct LogsModal;

impl LogsModal {
    const fn new() -> Self {
        Self
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        let area = area.with_size(area.size / 2).center(area);
        let bg = frame.palette().background().slight_offset().as_color();
        Block::fill(Style::bg(bg)).render(area, frame);
        Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, frame);

        frame.push_str(" Logs ");
        frame.render(area, TextOptions::span_center_top());
    }

    pub fn input(&mut self, key: Key) -> ModalAction {
        match key.code {
            KeyCode::Enter => ModalAction::Confirm,
            _ => ModalAction::None,
        }
    }
}
