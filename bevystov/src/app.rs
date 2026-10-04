use std::time::{Duration, Instant};

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
    state::{
        FreelyMutableState, NextState, OnEnter, OnExit, State, StateTransition,
        StateTransitionEvent, StateTransitionSystems, States,
    },
    state_scoped::{
        despawn_entities_on_enter_state, despawn_entities_on_exit_state,
        despawn_entities_when_state, disable_entities_on_enter_state,
        disable_entities_on_exit_state, disable_entities_when_state,
        enable_entities_on_enter_state, enable_entities_on_exit_state, enable_entities_when_state,
    },
};
use terminal::*;

use crate::{
    modals::{Modal, ModalsPlugin},
    pages::{Page, PagesPlugin, Route, TracksParam},
};

pub struct App {
    world: World,
    schedules: Schedules,
    is_running: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = {
            let mut world = World::new();
            let schedules = Schedules::new();
            bevy_state::state::setup_state_transitions_in_world(&mut world);
            Self {
                world,
                schedules,
                is_running: true,
            }
        };

        app.insert_resource(Frame::default())
            .insert_resource(Input::default())
            .insert_resource(Actions::default())
            .insert_resource(Colors::default())
            .insert_resource(PageArea::default());

        app.add_state(InputState::default());

        app.add_input(InputState::Normal, input_app)
            .add_render(RenderSet::App, render_app);

        PagesPlugin::build(&mut app);
        ModalsPlugin::build(&mut app);

        app
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> Result<(), Box<dyn std::error::Error>> {
        self.init(terminal)?;

        // Event loop
        let mut update = Timer::new(8);
        let mut render = Timer::new(1);

        while self.is_running {
            // Update at a fixed rate
            if update.tick() {
                self.run_update();
            }

            // Render at a fixed rate
            if render.tick() {
                self.run_render(terminal)?;
            }

            // Poll for events in a non-blocking manner
            if let Some(event) = Terminal::poll(update.timeout).unwrap() {
                match event {
                    TerminalEvent::Key(key) => {
                        self.insert_resource(Input(key));
                        self.run_input(terminal)?;
                    }
                    TerminalEvent::Resize => {
                        self.run_render(terminal)?;
                    }
                }
            };

            self.world.clear_trackers();
        }

        Ok(())
    }

    pub fn insert_resource(&mut self, resource: impl Resource) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_state<S: FreelyMutableState + Copy>(&mut self, state: S) -> &mut Self {
        // Insert state
        self.insert_resource(State::new(state))
            .insert_resource(NextState::<S>::default())
            .insert_resource(Messages::<StateTransitionEvent<S>>::default());

        // Register state
        let mut schedules = self.world.resource_mut::<bevy_ecs::schedule::Schedules>();
        let states = schedules.get_mut(StateTransition).unwrap();
        S::register_state(states);

        // Enable state scoped entities
        states
            .add_systems(
                (
                    despawn_entities_on_exit_state::<S>,
                    disable_entities_on_exit_state::<S>,
                    enable_entities_on_exit_state::<S>,
                )
                    .in_set(StateTransitionSystems::ExitSchedules),
            )
            .add_systems(
                (
                    despawn_entities_on_enter_state::<S>,
                    disable_entities_on_enter_state::<S>,
                    enable_entities_on_enter_state::<S>,
                )
                    .in_set(StateTransitionSystems::EnterSchedules),
            )
            .add_systems(
                (
                    despawn_entities_when_state::<S>,
                    disable_entities_when_state::<S>,
                    enable_entities_when_state::<S>,
                )
                    .in_set(StateTransitionSystems::TransitionSchedules),
            );

        // Send initial event
        self.world.write_message(StateTransitionEvent {
            exited: None,
            entered: Some(state),
            allow_same_state_transitions: false,
        });

        self
    }

    pub fn add_enter<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<bevy_ecs::schedule::Schedules>();
        let enter = schedules.entry(OnEnter(state));
        enter.add_systems(systems);
        self
    }

    pub fn add_exit<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<bevy_ecs::schedule::Schedules>();
        let exit = schedules.entry(OnExit(state));
        exit.add_systems(systems);
        self
    }

    pub fn add_input<M>(
        &mut self,
        state: InputState,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.schedules
            .input
            .add_systems(systems.run_if(in_state(state)));
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

    fn run_input(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        self.schedules.input.run(&mut self.world);

        if self.apply_actions() {
            self.run_state_transitions();
            self.run_render(terminal)?;
        }

        Ok(())
    }

    fn run_update(&mut self) {
        self.schedules.update.run(&mut self.world);
    }

    fn run_render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        terminal.render(|buffer| {
            let mut frame = self.world.resource_mut::<Frame>();
            frame.set_size(buffer.size().clone());

            self.schedules.render.run(&mut self.world);

            let mut frame = self.world.resource_mut::<Frame>();
            std::mem::swap(&mut frame.0, buffer);

            Ok(())
        })?;

        let mut frame = self.world.resource_mut::<Frame>();
        std::mem::swap(&mut frame.0, terminal.frame());

        Ok(())
    }

    fn run_state_transitions(&mut self) {
        self.world.run_schedule(StateTransition);
    }

    fn init(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        // Swap frames initially as terminal provides the proper one.
        // The frame inside bevy app is using dummy default values.
        // We will do double swap for every render.
        let mut frame = self.world.resource_mut::<Frame>();
        std::mem::swap(&mut frame.0, terminal.frame());

        // First run
        self.run_state_transitions();
        self.run_render(terminal)
    }

    fn apply_actions(&mut self) -> bool {
        let mut render = false;

        let mut actions = self.world.remove_resource::<Actions>().unwrap();

        for action in actions.drain(..) {
            match action {
                Action::Render => {
                    render = true;
                }
                Action::Route(route) => {
                    render = true;

                    let mut next_page = self.world.resource_mut::<NextState<Page>>();
                    next_page.set(route.as_page());

                    match route {
                        Route::Tracks(id) => {
                            let mut params = self.world.resource_mut::<TracksParam>();
                            **params = id;
                        }
                        Route::NowPlaying => {}
                        Route::Settings => {}
                    }
                }
                Action::Modal(modal) => {
                    render = true;

                    let mut next_modal = self.world.resource_mut::<NextState<Modal>>();
                    next_modal.set(modal);

                    let mut next_input = self.world.resource_mut::<NextState<InputState>>();
                    next_input.set(InputState::from(modal));
                }
                Action::Quit => {
                    self.is_running = false;
                }
            }
        }

        self.world.insert_resource(actions);

        render
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
        render.configure_sets((RenderSet::App, RenderSet::Page, RenderSet::Modal).chain());

        Self {
            input,
            update,
            render,
        }
    }
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct Frame(Framebuffer);

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum RenderSet {
    App,
    Page,
    Modal,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum InputState {
    #[default]
    Normal,
    Modal,
}

impl InputState {
    const fn from(modal: Modal) -> Self {
        match modal {
            Modal::Search | Modal::Logs | Modal::Custom => Self::Modal,
            Modal::Confirmed | Modal::Canceled => Self::Normal,
        }
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Render,
    Route(Route),
    Modal(Modal),
    // Clear,
    Quit,
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct Actions(Vec<Action>);

#[derive(Debug, Clone, Resource)]
pub struct Colors {
    pub normal: Color,
    pub primary: Color,
    pub secondary: Color,
    pub neutral: Color,
    pub red: Color,
}

impl Colors {
    const fn _all(color: Color) -> Self {
        Self {
            normal: color,
            primary: color,
            secondary: color,
            neutral: color,
            red: color,
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            normal: Color::Default,
            primary: Color::BrightYellow,
            secondary: Color::Yellow,
            neutral: Color::Indexed(240),
            red: Color::Red,
        }
    }
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct PageArea(Rect);

fn input_app(key: Res<Input>, mut actions: ResMut<Actions>, page: Res<State<Page>>) {
    match key.code {
        KeyCode::Esc => actions.push(Action::Quit),
        KeyCode::Tab => actions.push(Action::Route(page.next())),
        KeyCode::BackTab => actions.push(Action::Route(page.prev())),
        KeyCode::Char('f') if key.ctrl() => {
            actions.push(Action::Modal(Modal::Search));
        }
        KeyCode::Char('l') if key.ctrl() => {
            actions.push(Action::Modal(Modal::Logs));
        }
        _ => {}
    }
}

fn render_app(mut frame: ResMut<Frame>, mut page_area: ResMut<PageArea>, colors: Res<Colors>) {
    let area = frame.area();

    let (top, body, bottom) = area.split_ends(1, 1);

    // Navigation
    frame.push_str_fg("TODO TOP", colors.normal);
    frame.render(top, TextOptions::span_center_top());

    // Setup area for pages
    **page_area = body.inner(Margin::all(1));

    // Global shortcuts
    frame.push_str_fg("TODO BOTTOM", colors.normal);
    frame.render(bottom, TextOptions::span_center_top());
}
