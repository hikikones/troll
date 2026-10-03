use bevy_app::{App, AppExit};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    message::MessageCursor, prelude::*, schedule::ScheduleLabel, system::ScheduleSystem,
};
use bevy_state::{
    app::AppExtStates,
    condition::in_state,
    state::{
        FreelyMutableState, NextState, OnEnter, OnExit, State, StateTransition,
        StateTransitionEvent, StateTransitionSystems, States,
    },
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

fn app_runner(mut app: App) -> AppExit {
    // First render
    let world = app.world_mut();
    world.run_schedule(StateTransition);
    world.run_schedule(MySchedule::Render);

    // Setup timers
    let mut update = Timer::new(8);
    let mut render = Timer::new(1);

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

// struct MainSchedule(Schedule);

// impl MainSchedule {
//     fn new() -> Self {
//         Self(Schedule::default())
//     }
// }

// struct Schedules {
//     main: Schedule,
//     states: bevy_ecs::schedule::Schedules,
// }

// impl Schedules {
//     fn new(world: &mut World) -> Self {
//         let mut transition = Schedule::new(StateTransition);
//         transition.configure_sets(
//             (
//                 StateTransitionSystems::DependentTransitions,
//                 StateTransitionSystems::ExitSchedules,
//                 StateTransitionSystems::TransitionSchedules,
//                 StateTransitionSystems::EnterSchedules,
//             )
//                 .chain(),
//         );

//         let mut states = bevy_ecs::schedule::Schedules::new();
//         states.insert(transition);

//         Self {
//             main: Schedule::default(),
//             states,
//         }
//     }
// }

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

    pub fn add_input<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.input.add_systems(systems);
        self
    }

    pub fn add_render<M>(
        &mut self,
        set: RenderSet,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.render.add_systems(systems.in_set(set));
        self
    }

    fn run_input(&mut self, world: &mut World) {
        self.input.run(world);
        world.run_schedule(StateTransition);

        // TODO: check for event for explicit render
    }

    fn run_update(&mut self, world: &mut World) {
        self.update.run(world);
    }

    fn run_render(&mut self, world: &mut World) {
        self.render.run(world);
    }
}

fn add_state<S: FreelyMutableState + Default>(world: &mut World) {
    world.insert_resource(State::new(S::default()));
    world.insert_resource(NextState::<S>::default());
    world.insert_resource(Messages::<StateTransitionEvent<S>>::default());

    S::register_state(
        world
            .resource_mut::<bevy_ecs::schedule::Schedules>()
            .get_mut(StateTransition)
            .unwrap(),
    );

    world.write_message(StateTransitionEvent {
        exited: None,
        entered: Some(S::default()),
        allow_same_state_transitions: false,
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = Output::new();
    let buffer = Framebuffer::new(&mut output.0)?;

    Terminal::enter(|| {
        // Setup ECS
        let mut world = World::new();
        let mut schedules = Schedules::new();
        bevy_state::state::setup_state_transitions_in_world(&mut world);

        world.insert_resource(output);
        world.insert_resource(buffer);
        world.insert_resource(Input::default());

        add_state::<AppState>(&mut world);

        schedules
            .add_input((
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

        // First render
        world.run_schedule(StateTransition);
        schedules.run_render(&mut world);

        // Setup timers
        let mut update = Timer::new(8);
        let mut render = Timer::new(1);

        // Event loop
        loop {
            // Update at a fixed rate
            if update.tick() {
                schedules.run_update(&mut world);
            }

            // Render at a fixed rate
            if render.tick() {
                schedules.run_render(&mut world);
            }

            // Poll for events in a non-blocking manner
            if let Some(event) = Terminal::poll(update.timeout).unwrap() {
                match event {
                    TerminalEvent::Key(key) => {
                        if let KeyCode::Esc = key.code {
                            break;
                        }

                        world.insert_resource(Input(key));
                        schedules.run_input(&mut world);

                        // TODO: Check for app event produced by input for explicit render
                        schedules.run_render(&mut world);
                    }
                    TerminalEvent::Resize => {
                        schedules.run_render(&mut world);
                    }
                }
            };

            // world.clear_trackers();
        }

        Ok(())
    })

    // Terminal::enter(|| {
    //     let mut app = App::empty();

    //     app.insert_resource(output)
    //         .insert_resource(buffer)
    //         .init_resource::<Input>()
    //         .init_schedule(MySchedule::Input)
    //         .init_schedule(MySchedule::Update)
    //         .init_schedule(MySchedule::Render)
    //         .configure_sets(
    //             MySchedule::Render,
    //             (RenderSet::Clear, RenderSet::Build, RenderSet::Flush).chain(),
    //         )
    //         .init_schedule(StateTransition)
    //         .edit_schedule(StateTransition, |schedule| {
    //             schedule.configure_sets(
    //                 (
    //                     StateTransitionSystems::DependentTransitions,
    //                     StateTransitionSystems::ExitSchedules,
    //                     StateTransitionSystems::TransitionSchedules,
    //                     StateTransitionSystems::EnterSchedules,
    //                 )
    //                     .chain(),
    //             );
    //         })
    //         .init_state::<AppState>()
    //         .add_message::<AppExit>()
    //         .add_systems(OnEnter(AppState::Menu), enter_menu)
    //         .add_systems(OnEnter(AppState::Game), enter_game)
    //         .add_systems(OnExit(AppState::Menu), exit_menu)
    //         .add_systems(OnExit(AppState::Game), exit_game)
    //         .add_systems(MySchedule::Input, menu.run_if(in_state(AppState::Menu)))
    //         .add_systems(MySchedule::Input, game.run_if(in_state(AppState::Game)))
    //         .add_systems(MySchedule::Input, quit)
    //         .add_systems(
    //             MySchedule::Render,
    //             update_menu
    //                 .in_set(RenderSet::Build)
    //                 .run_if(in_state(AppState::Menu)),
    //         )
    //         .add_systems(
    //             MySchedule::Render,
    //             update_game
    //                 .in_set(RenderSet::Build)
    //                 .run_if(in_state(AppState::Game)),
    //         )
    //         .add_systems(MySchedule::Render, flush.in_set(RenderSet::Flush));

    //     app.set_runner(app_runner).run();

    //     Ok(())
    // })
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

fn enter_menu(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("enter menu");
}

fn exit_menu(mut frame: ResMut<Framebuffer>) {
    // frame.print_str("exit menu");
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
