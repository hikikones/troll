use std::time::{Duration, Instant};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    message::Messages,
    resource::Resource,
    schedule::{IntoScheduleConfigs, Schedule, SystemSet},
    system::{NonSend, NonSendMut, Res, ResMut, ScheduleSystem},
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
    database::{Database, DatabaseEvent},
    jukebox::{Jukebox, JukeboxEvent},
    modals::{Modal, ModalsPlugin},
    pages::{Page, PagesPlugin, Route, TracksPage},
};

pub struct App {
    world: World,
    schedules: Schedules,
    is_running: bool,
}

impl App {
    pub fn new(database: Database, jukebox: Jukebox) -> Self {
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

        app.insert_non_send(database);
        app.insert_non_send(jukebox);

        let colors = Colors::default();
        let modal_colors = ModalColors(Colors::all(colors.neutral));

        app.insert_resource(Frame::default())
            .insert_resource(Input::default())
            .insert_resource(Actions::default())
            .insert_resource(colors)
            .insert_resource(modal_colors)
            .insert_resource(PageArea::default())
            .insert_resource(ShortcutsArea::default());

        app.add_state(InputState::default());

        app.add_render(RenderSet::SetColors, swap_colors)
            .add_render(RenderSet::App, render_app)
            .add_render(RenderSet::ResetColors, swap_colors)
            .add_update(update_app);

        app.schedules
            .input
            .add_systems(input_app.in_set(InputSet::Global));

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
                self.run_update(terminal)?;
            }

            // Render at a fixed rate
            if render.tick() {
                self.run_render(terminal)?;
            }

            // Poll for events in a non-blocking manner
            if let Some(event) = Terminal::poll(update.timeout).unwrap() {
                match event {
                    TerminalEvent::Key(key) => {
                        self.insert_resource(Input(Some(key)));
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

    pub fn insert_resource(&mut self, value: impl Resource) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    pub fn insert_non_send<R: 'static>(&mut self, value: R) -> &mut Self {
        self.world.insert_non_send(value);
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
            .add_systems(systems.in_set(InputSet::Normal).run_if(in_state(state)));
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

    fn run_update(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        self.schedules.update.run(&mut self.world);

        if self.apply_actions() {
            self.run_render(terminal)?;
        }

        Ok(())
    }

    fn run_render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        terminal.render(|buffer| {
            let mut frame = self.world.resource_mut::<Frame>();
            frame.query_term_size()?;

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

        // Start database load
        self.world.non_send_mut::<Database>().load();

        // First run
        self.run_state_transitions();
        self.run_render(terminal)
    }

    fn apply_actions(&mut self) -> bool {
        if self.world.resource::<Actions>().is_empty() {
            return false;
        }

        let mut render = false;
        let mut actions = self.world.remove_resource::<Actions>().unwrap();

        for action in actions.drain(..) {
            match action {
                Action::Render => {
                    render = true;
                }
                Action::Route(route) => {
                    render = true;

                    let modal = *self.world.resource::<State<Modal>>().get();
                    if modal.is_active() {
                        self.world
                            .resource_mut::<NextState<Modal>>()
                            .set(Modal::Canceled);
                        self.world
                            .resource_mut::<NextState<InputState>>()
                            .set(InputState::Normal);
                    } else {
                        self.world
                            .resource_mut::<NextState<Page>>()
                            .set(route.as_page());

                        match route {
                            Route::Tracks(id) => {
                                let mut page = self.world.resource_mut::<TracksPage>();
                                page.set_params(id);
                            }
                            Route::NowPlaying => {}
                            Route::Settings => {}
                        }
                    }
                }
                Action::Modal(modal) => {
                    let current = *self.world.resource::<State<Modal>>().get();
                    let next = match (current, modal) {
                        (Modal::Search, Modal::Search) => Modal::Canceled,
                        (Modal::Logs, Modal::Logs) => Modal::Canceled,
                        (_, _) => modal,
                    };

                    if current != next {
                        render = true;
                        self.world.resource_mut::<NextState<Modal>>().set(next);
                        self.world
                            .resource_mut::<NextState<InputState>>()
                            .set(next.as_input_state());
                    }
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
        let mut input = Schedule::default();
        input.configure_sets((InputSet::Global, InputSet::Normal).chain());

        let mut render = Schedule::default();
        render.configure_sets(
            (
                RenderSet::SetColors,
                RenderSet::App,
                RenderSet::Page,
                RenderSet::ResetColors,
                RenderSet::Modal,
            )
                .chain(),
        );

        Self {
            input,
            render,
            update: Schedule::default(),
        }
    }
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct Frame(Framebuffer);

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum RenderSet {
    SetColors,
    App,
    Page,
    ResetColors,
    Modal,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
enum InputSet {
    Global,
    Normal,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum InputState {
    #[default]
    Normal,
    Modal,
}

#[derive(Debug, Default, Resource)]
pub struct Input(Option<Key>);

impl Input {
    const fn take(&mut self) -> Key {
        self.0.take().unwrap()
    }

    const fn set(&mut self, key: Key) {
        self.0 = Some(key);
    }

    pub const fn get(&self) -> Option<Key> {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Render,
    Route(Route),
    Modal(Modal),
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
    const fn all(color: Color) -> Self {
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

#[derive(Debug, Clone, Resource, Deref, DerefMut)]
struct ModalColors(Colors);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct PageArea(Rect);

#[derive(Default, Resource, Deref, DerefMut)]
pub struct ShortcutsArea(Rect);

fn input_app(
    mut input: ResMut<Input>,
    mut actions: ResMut<Actions>,
    page: Res<State<Page>>,
    mut jukebox: NonSendMut<Jukebox>,
    database: NonSend<Database>,
) {
    let key = input.take();

    match key.code {
        KeyCode::Esc => actions.push(Action::Quit),
        KeyCode::Tab => actions.push(Action::Route(page.next())),
        KeyCode::BackTab => actions.push(Action::Route(page.prev())),
        KeyCode::Down if key.ctrl() => {
            jukebox.stop();
        }
        KeyCode::Up if key.ctrl() => {
            jukebox.pause_or_play();
        }
        KeyCode::Right if key.ctrl() => {
            jukebox.play_next(&database);
        }
        KeyCode::Left if key.ctrl() => {
            jukebox.play_previous(&database);
        }
        KeyCode::Char('f') if key.ctrl() => {
            actions.push(Action::Modal(Modal::Search));
        }
        KeyCode::Char('l') if key.ctrl() => {
            actions.push(Action::Modal(Modal::Logs));
        }
        _ => {
            input.set(key);
        }
    }
}

fn swap_colors(
    modal: Res<State<Modal>>,
    mut colors: ResMut<Colors>,
    mut modal_colors: ResMut<ModalColors>,
) {
    if modal.is_active() {
        std::mem::swap(&mut *colors, &mut **modal_colors);
    }
}

fn render_app(
    mut frame: ResMut<Frame>,
    mut page_area: ResMut<PageArea>,
    mut shortcuts_area: ResMut<ShortcutsArea>,
    colors: Res<Colors>,
    current_page: Res<State<Page>>,
) {
    let area = frame.area();

    match frame.screen_height() {
        ScreenHeight::Short => {
            **page_area = area;
            **shortcuts_area = Rect::ZERO;
        }
        ScreenHeight::Normal => {
            let (top, body, bottom) = area.split_ends(1, 1);

            render_navigation(top, &mut frame, &colors, **current_page);

            **page_area = body.inner(Margin::all(1));
            **shortcuts_area = bottom;
        }
        ScreenHeight::Tall => {
            let (top, body, mut bottom) = area.split_ends(1, 6);

            render_navigation(top, &mut frame, &colors, **current_page);

            **page_area = body.inner(Margin::all(1));
            **shortcuts_area = bottom.with_rows(1);

            bottom.shrink_down(2);

            let title_area = bottom.with_rows(1);
            bottom.shrink_down(1);
            let playback_area = bottom.with_rows(1);
            bottom.shrink_down(1);
            let play_shortcuts_area = bottom.with_rows(1);
            bottom.shrink_down(1);
            let app_shortcuts_area = bottom.with_rows(1);

            frame.push_str_fg("TODO BOTTOM", colors.normal);
            frame.render(app_shortcuts_area, TextOptions::span_center_top());
        }
    }
}

fn render_navigation(area: Rect, frame: &mut Framebuffer, colors: &Colors, current_page: Page) {
    for (page, name, gap) in [
        (Page::Tracks, "Tracks", 3),
        (Page::NowPlaying, "Now Playing", 3),
        (Page::Settings, "Settings", 0),
    ] {
        if current_page == page {
            frame.push_fmt(Styled::new(name, Style::fg(colors.primary).with_bold()));
        } else {
            frame.push_str_fg(name, colors.normal);
        }
        frame.push_ch_repeat(' ', gap);
    }
    frame.render(area, TextOptions::span_center_top());
}

fn update_app(
    mut database: NonSendMut<Database>,
    mut jukebox: NonSendMut<Jukebox>,
    mut actions: ResMut<Actions>,
) {
    let mut render = false;

    database.update(|event| {
        render = true;

        match event {
            DatabaseEvent::Rating(_id) => {
                // TODO
            }
            DatabaseEvent::Error(_err) => {
                // TODO
            }
        }
    });

    jukebox.update(&database, |event| {
        render = true;

        match event {
            JukeboxEvent::Play(_id) => {
                // TODO
            }
            JukeboxEvent::Pause => {
                // TODO
            }
            JukeboxEvent::Stop => {
                // TODO
            }
            JukeboxEvent::Error(_err) => {
                // TODO
            }
        }
    });

    if render {
        actions.push(Action::Render);
    }
}
