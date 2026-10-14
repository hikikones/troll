use bevy_ecs::{
    message::Messages,
    schedule::{IntoScheduleConfigs, Schedules},
    system::ScheduleSystem,
    world::World,
};
use bevy_state::{
    state::{
        FreelyMutableState, NextState, OnEnter, OnExit, State, StateTransition,
        StateTransitionEvent, StateTransitionSystems, States,
    },
    state_scoped,
};

pub trait WorldExt {
    fn add_state<S: FreelyMutableState + Copy>(&mut self, state: S);
    fn add_enter<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    );
    fn add_exit<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    );
}

impl WorldExt for World {
    fn add_state<S: FreelyMutableState + Copy>(&mut self, state: S) {
        // Insert state
        self.insert_resource(State::new(state));
        self.insert_resource(NextState::<S>::default());
        self.insert_resource(Messages::<StateTransitionEvent<S>>::default());

        // Register state
        let mut schedules = self.resource_mut::<Schedules>();
        let states = schedules.get_mut(StateTransition).unwrap();
        S::register_state(states);

        // Enable state scoped entities
        states
            .add_systems(
                (
                    state_scoped::despawn_entities_on_exit_state::<S>,
                    state_scoped::disable_entities_on_exit_state::<S>,
                    state_scoped::enable_entities_on_exit_state::<S>,
                )
                    .in_set(StateTransitionSystems::ExitSchedules),
            )
            .add_systems(
                (
                    state_scoped::despawn_entities_on_enter_state::<S>,
                    state_scoped::disable_entities_on_enter_state::<S>,
                    state_scoped::enable_entities_on_enter_state::<S>,
                )
                    .in_set(StateTransitionSystems::EnterSchedules),
            )
            .add_systems(
                (
                    state_scoped::despawn_entities_when_state::<S>,
                    state_scoped::disable_entities_when_state::<S>,
                    state_scoped::enable_entities_when_state::<S>,
                )
                    .in_set(StateTransitionSystems::TransitionSchedules),
            );

        // Send initial event
        self.write_message(StateTransitionEvent {
            exited: None,
            entered: Some(state),
            allow_same_state_transitions: true,
        });
    }

    fn add_enter<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        let mut schedules = self.resource_mut::<Schedules>();
        let enter = schedules.entry(OnEnter(state));
        enter.add_systems(systems);
    }

    fn add_exit<S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        let mut schedules = self.resource_mut::<Schedules>();
        let exit = schedules.entry(OnExit(state));
        exit.add_systems(systems);
    }
}
