pub use bevy_derive::{Deref, DerefMut};
pub use bevy_ecs::{prelude::*, system::ScheduleSystem};
pub use bevy_state::{
    prelude::*,
    state::{FreelyMutableState, setup_state_transitions_in_world},
};

mod extensions;

pub use extensions::*;
