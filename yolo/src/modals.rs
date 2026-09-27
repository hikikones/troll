#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Confirm,
    Custom,
}

#[derive(Debug, Clone, Copy)]
pub enum ModalAction {
    None,
    Render,
    Confirm,
    Cancel,
}
