use std::time::{Duration, Instant};

use terminal::{Terminal, TerminalEvent};

type Sender = std::sync::mpsc::Sender<Event>;
type Receiver = std::sync::mpsc::Receiver<Event>;

pub enum Event {
    Update,
    Render,
    Terminal(TerminalEvent),
}

pub struct EventHandler {
    sender: Sender,
    receiver: Receiver,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self { sender, receiver }
    }

    pub fn start(&self) {
        let sender = self.sender.clone();
        std::thread::spawn(move || handle_terminal_events(sender));
    }

    pub fn next(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        Ok(self.receiver.recv()?)
    }
}

fn handle_terminal_events(sender: Sender) -> Result<(), std::io::Error> {
    const UPDATE_FREQUENCY: f64 = 1.0 / 8.0;
    const RENDER_FREQUENCY: f64 = 1.0 / 1.0;

    // Setup timers
    let mut update = Timer::new(Duration::from_secs_f64(UPDATE_FREQUENCY));
    let mut render = Timer::new(Duration::from_secs_f64(RENDER_FREQUENCY));

    loop {
        // Update at a fixed rate
        if update.tick() {
            let _ = sender.send(Event::Update);
        }

        // Render at a fixed rate
        if render.tick() {
            let _ = sender.send(Event::Render);
        }

        // Poll for events in a non-blocking manner
        if let Some(event) = Terminal::poll(update.timeout())? {
            let _ = sender.send(Event::Terminal(event));
        }
    }
}

struct Timer {
    interval: Duration,
    last_tick: Instant,
    timeout: Duration,
}

impl Timer {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_tick: Instant::now(),
            timeout: Duration::ZERO,
        }
    }

    fn tick(&mut self) -> bool {
        self.timeout = self.interval.saturating_sub(self.last_tick.elapsed());
        if self.timeout == Duration::ZERO {
            self.last_tick = Instant::now();
            true
        } else {
            false
        }
    }

    const fn timeout(&self) -> Duration {
        self.timeout
    }
}
