use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    message::Messages,
    resource::Resource,
    schedule::{IntoScheduleConfigs, Schedule, SystemSet},
    system::{Res, ResMut, ScheduleSystem},
    world::World,
};
use bevy_state::{
    condition::in_state,
    state::{FreelyMutableState, NextState, State, StateTransition, StateTransitionEvent, States},
};
use terminal::{Key, KeyCode, KeyModifiers, TerminalEvent};

use std::time::{Duration, Instant};

pub struct App {
    world: World,
    schedules: Schedules,
}

impl App {
    pub fn new(terminal: Terminal, framebuffer: Framebuffer) -> Self {
        let mut app = {
            let mut world = World::new();
            let schedules = Schedules::new();
            bevy_state::state::setup_state_transitions_in_world(&mut world);
            Self { world, schedules }
        };

        app.insert_resource(terminal)
            .insert_resource(framebuffer)
            .insert_resource(Input::default());

        app.add_state(AppState::default());

        app.add_input((
            input_menu.run_if(in_state(AppState::Menu)),
            input_game.run_if(in_state(AppState::Game)),
        ))
        .add_render(
            RenderSet::Build,
            (
                render_menu.run_if(in_state(AppState::Menu)),
                render_game.run_if(in_state(AppState::Game)),
            ),
        )
        .add_render(RenderSet::Flush, flush);

        // TODO: plugins

        app
    }

    pub fn insert_resource(&mut self, resource: impl Resource) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_state<S: FreelyMutableState + Copy>(&mut self, state: S) -> &mut Self {
        self.insert_resource(State::new(state))
            .insert_resource(NextState::<S>::default())
            .insert_resource(Messages::<StateTransitionEvent<S>>::default());

        S::register_state(
            self.world
                .resource_mut::<bevy_ecs::schedule::Schedules>()
                .get_mut(StateTransition)
                .unwrap(),
        );

        self.world.write_message(StateTransitionEvent {
            exited: None,
            entered: Some(state),
            allow_same_state_transitions: false,
        });

        self
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // First run
        self.run_state_transitions();
        self.run_render();

        // Event loop
        let mut update = Timer::new(8);
        let mut render = Timer::new(1);

        loop {
            // Update at a fixed rate
            if update.tick() {
                self.run_update();
            }

            // Render at a fixed rate
            if render.tick() {
                self.run_render();
            }

            // Poll for events in a non-blocking manner
            if let Some(event) = terminal::bevy::Terminal::poll(update.timeout).unwrap() {
                match event {
                    TerminalEvent::Key(key) => {
                        if let KeyCode::Esc = key.code {
                            break;
                        }

                        self.insert_resource(Input(key));
                        self.run_input();
                    }
                    TerminalEvent::Resize => {
                        self.run_render();
                    }
                }
            };

            self.world.clear_trackers();
        }

        Ok(())
    }

    pub fn add_input<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.schedules.input.add_systems(systems);
        self
    }

    pub fn add_update<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.schedules.update.add_systems(systems);
        self
    }

    pub fn add_render<M>(
        &mut self,
        set: RenderSet,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.schedules.render.add_systems(systems.in_set(set));
        self
    }

    fn run_input(&mut self) {
        self.schedules.input.run(&mut self.world);
        self.run_state_transitions();

        // TODO: only render if input produces a render event
        self.run_render();
    }

    fn run_update(&mut self) {
        self.schedules.update.run(&mut self.world);
    }

    fn run_render(&mut self) {
        self.schedules.render.run(&mut self.world);
    }

    fn run_state_transitions(&mut self) {
        self.world.run_schedule(StateTransition);
    }
}

struct Timer {
    interval: Duration,
    last_tick: Instant,
    timeout: Duration,
}

impl Timer {
    fn new(fps: u8) -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / fps as f64),
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
}

struct Schedules {
    input: Schedule,
    update: Schedule,
    render: Schedule,
}

impl Schedules {
    fn new() -> Self {
        let input = Schedule::default();
        let update = Schedule::default();
        let mut render = Schedule::default();
        render.configure_sets((RenderSet::Clear, RenderSet::Build, RenderSet::Flush).chain());

        Self {
            input,
            update,
            render,
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct Terminal(std::io::Stdout);

impl Terminal {
    pub fn new() -> Self {
        Self(std::io::stdout())
    }

    pub fn lock(&self) -> std::io::StdoutLock<'_> {
        self.0.lock()
    }
}

impl std::io::Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct Framebuffer(terminal::bevy::Framebuffer);

impl Framebuffer {
    pub fn new(writer: &mut impl std::io::Write) -> std::io::Result<Self> {
        let buf = terminal::bevy::Framebuffer::query(writer, &mut std::io::stdin())?;
        Ok(Self(buf))
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSet {
    Clear,
    Build,
    Flush,
}

#[derive(Resource, Deref)]
pub struct Input(Key);

impl Default for Input {
    fn default() -> Self {
        Self(Key {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum AppState {
    #[default]
    Menu,
    Game,
}

fn input_menu(input: Res<Input>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Game);
        }
        _ => {}
    }
}

fn render_menu(mut frame: ResMut<Framebuffer>) {
    frame.print_str("menu");
}

fn input_game(input: Res<Input>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Menu);
        }
        _ => {}
    }
}

fn render_game(mut frame: ResMut<Framebuffer>) {
    frame.print_str("game");
}

fn flush(mut frame: ResMut<Framebuffer>, mut output: ResMut<Terminal>) {
    terminal::bevy::Terminal::render(&mut frame, &mut output.0).unwrap();
}
