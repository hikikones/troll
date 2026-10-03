use bevy_app::{App, AppExit};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{message::MessageCursor, prelude::*, schedule::ScheduleLabel};
use bevy_state::{
    app::AppExtStates,
    condition::in_state,
    state::{NextState, OnEnter, OnExit, StateTransition, StateTransitionSystems, States},
};
use terminal::{Key, KeyCode, KeyModifiers, TerminalEvent, bevy::Terminal};

use std::time::{Duration, Instant};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    Menu,
    Game,
}

#[derive(Resource, Deref)]
struct Input(Key);

impl Default for Input {
    fn default() -> Self {
        Self(Key {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        })
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum MySchedule {
    Update,
    Input,
    Render,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum RenderSet {
    Clear,
    Build,
    Flush,
}

#[derive(Resource, Deref, DerefMut)]
struct Output(std::io::Stdout);

impl Output {
    fn new() -> Self {
        Self(std::io::stdout())
    }
}

impl std::io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[derive(Resource, Deref, DerefMut)]
struct Framebuffer(terminal::bevy::Framebuffer);

impl Framebuffer {
    fn new(writer: &mut impl std::io::Write) -> std::io::Result<Self> {
        let buf = terminal::bevy::Framebuffer::query(writer, &mut std::io::stdin())?;
        Ok(Self(buf))
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
}

fn app_runner(mut app: App) -> AppExit {
    // First render
    let world = app.world_mut();
    world.run_schedule(StateTransition);
    world.run_schedule(MySchedule::Render);

    // Setup timers
    const UPDATE_FREQUENCY: f64 = 1.0 / 8.0;
    const RENDER_FREQUENCY: f64 = 1.0 / 1.0;

    let mut update = Timer::new(Duration::from_secs_f64(UPDATE_FREQUENCY));
    let mut render = Timer::new(Duration::from_secs_f64(RENDER_FREQUENCY));

    loop {
        // Update at a fixed rate
        if update.tick() {
            world.run_schedule(MySchedule::Update);
        }

        // Render at a fixed rate
        if render.tick() {
            world.run_schedule(MySchedule::Render);
        }

        // Poll for events in a non-blocking manner
        if let Some(event) = Terminal::poll(update.timeout).unwrap() {
            match event {
                TerminalEvent::Key(key) => {
                    world.insert_resource(Input(key));
                    world.run_schedule(MySchedule::Input);
                    world.run_schedule(StateTransition);
                    world.run_schedule(MySchedule::Render);
                }
                TerminalEvent::Resize => todo!(),
            }
        };

        // Check for app exit
        let mut reader = MessageCursor::default();
        let messages = world.get_resource::<Messages<AppExit>>().unwrap();
        let mut messages = reader.read(messages);

        if messages.len() != 0 {
            return messages
                .find(|exit| exit.is_error())
                .cloned()
                .unwrap_or(AppExit::Success);
        }

        world.clear_trackers();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = Output::new();
    let buffer = Framebuffer::new(&mut output)?;

    Terminal::enter(|| {
        let mut app = App::empty();

        app.insert_resource(output)
            .insert_resource(buffer)
            .init_resource::<Input>()
            .init_schedule(MySchedule::Input)
            .init_schedule(MySchedule::Update)
            .init_schedule(MySchedule::Render)
            .configure_sets(
                MySchedule::Render,
                (RenderSet::Clear, RenderSet::Build, RenderSet::Flush).chain(),
            )
            .init_schedule(StateTransition)
            .edit_schedule(StateTransition, |schedule| {
                schedule.configure_sets(
                    (
                        StateTransitionSystems::DependentTransitions,
                        StateTransitionSystems::ExitSchedules,
                        StateTransitionSystems::TransitionSchedules,
                        StateTransitionSystems::EnterSchedules,
                    )
                        .chain(),
                );
            })
            .init_state::<AppState>()
            .add_message::<AppExit>()
            .add_systems(OnEnter(AppState::Menu), enter_menu)
            .add_systems(OnEnter(AppState::Game), enter_game)
            .add_systems(OnExit(AppState::Menu), exit_menu)
            .add_systems(OnExit(AppState::Game), exit_game)
            .add_systems(MySchedule::Input, menu.run_if(in_state(AppState::Menu)))
            .add_systems(MySchedule::Input, game.run_if(in_state(AppState::Game)))
            .add_systems(MySchedule::Input, quit)
            .add_systems(
                MySchedule::Render,
                update_menu
                    .in_set(RenderSet::Build)
                    .run_if(in_state(AppState::Menu)),
            )
            .add_systems(
                MySchedule::Render,
                update_game
                    .in_set(RenderSet::Build)
                    .run_if(in_state(AppState::Game)),
            )
            .add_systems(MySchedule::Render, flush.in_set(RenderSet::Flush));

        app.set_runner(app_runner).run();

        Ok(())
    })
}

fn menu(input: Res<Input>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Game);
        }
        _ => {}
    }
}

fn update_menu(mut frame: ResMut<Framebuffer>) {
    frame.print_str("menu");
}

fn enter_menu(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("enter menu");
}

fn exit_menu(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("exit menu");
}

fn game(input: Res<Input>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Menu);
        }
        _ => {}
    }
}

fn update_game(mut frame: ResMut<Framebuffer>) {
    frame.print_str("game");
}

fn enter_game(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("enter game");
}

fn exit_game(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("exit game");
}

fn quit(input: Res<Input>, mut writer: MessageWriter<AppExit>) {
    if let KeyCode::Esc = input.code {
        writer.write(AppExit::Success);
    }
}

fn flush(mut frame: ResMut<Framebuffer>, mut output: ResMut<Output>) {
    Terminal::render(&mut frame, &mut output.0).unwrap();
}
