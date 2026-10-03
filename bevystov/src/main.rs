use bevy_app::{App, AppExit};
use bevy_derive::Deref;
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_state::{
    app::AppExtStates,
    condition::in_state,
    state::{NextState, OnEnter, OnExit, StateTransition, StateTransitionSystems, States},
};

use std::time::Duration;
use terminal::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    Menu,
    Game,
}

#[derive(Message, Resource, Deref)]
struct TerminalInput(terminal::Key);

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum MySchedule {
    Main,
    Input,
    Render,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = terminal::Terminal::new()?;

    terminal.enter(|_term| {
        let mut app = App::empty();

        app.add_plugins((
            // ScheduleRunnerPlugin::run_loop(Duration::from_millis(100)),
            // StatesPlugin,
        ))
        .init_schedule(MySchedule::Input)
        .init_schedule(MySchedule::Main)
        .init_schedule(MySchedule::Render)
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
        .add_message::<AppExit>()
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Menu), enter_menu)
        .add_systems(OnEnter(AppState::Game), enter_game)
        .add_systems(OnExit(AppState::Menu), exit_menu)
        .add_systems(OnExit(AppState::Game), exit_game)
        .add_systems(MySchedule::Input, menu.run_if(in_state(AppState::Menu)))
        .add_systems(MySchedule::Input, game.run_if(in_state(AppState::Game)))
        .add_systems(MySchedule::Input, quit)
        .add_systems(
            MySchedule::Main,
            update_menu.run_if(in_state(AppState::Menu)),
        )
        .add_systems(
            MySchedule::Main,
            update_game.run_if(in_state(AppState::Game)),
        );

        app.set_runner(|mut app| {
            app.insert_resource(TerminalInput(Key {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::empty(),
            }));

            app.world_mut().run_schedule(StateTransition);
            app.world_mut().run_schedule(MySchedule::Main);
            app.world_mut().run_schedule(MySchedule::Render);

            loop {
                if let Some(event) = Terminal::poll(Duration::from_secs(1)).unwrap() {
                    match event {
                        TerminalEvent::Key(key) => {
                            app.insert_resource(TerminalInput(key));
                            app.world_mut().run_schedule(MySchedule::Input);
                        }
                        TerminalEvent::Resize => todo!(),
                    }
                };

                print!("\x1b[2J\x1b[1;1H");
                app.world_mut().run_schedule(StateTransition);
                app.world_mut().run_schedule(MySchedule::Main);
                app.world_mut().run_schedule(MySchedule::Render);

                app.world_mut().clear_trackers();

                if let Some(exit) = app.should_exit() {
                    return exit;
                }
            }
        })
        .run();

        Ok(())
    })
}

fn menu(input: Res<TerminalInput>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Game);
        }
        _ => {}
    }
}

fn update_menu() {
    println!("menu");
}

fn enter_menu() {
    println!("enter menu");
}

fn exit_menu() {
    println!("exit menu");
}

fn game(input: Res<TerminalInput>, mut next_state: ResMut<NextState<AppState>>) {
    match input.code {
        KeyCode::Enter => {
            next_state.set(AppState::Menu);
        }
        _ => {}
    }
}

fn update_game() {
    println!("game");
}

fn enter_game() {
    println!("enter game");
}

fn exit_game() {
    println!("exit game");
}

fn quit(input: Res<TerminalInput>, mut writer: MessageWriter<AppExit>) {
    if let KeyCode::Esc = input.code {
        writer.write(AppExit::Success);
    }
}
