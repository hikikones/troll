use shared::time::UnixTime;

pub struct Scheduler(fsrs::FSRS);

impl Scheduler {
    pub fn new() -> Self {
        Self(fsrs::FSRS::default())
    }

    pub fn schedule(
        &self,
        state: CurrentReviewState,
        success: bool,
        desired_retention: f32,
    ) -> NextReviewState {
        let memory_state = if state.stability == 0.0 || state.difficulty == 0.0 {
            None
        } else {
            Some(fsrs::MemoryState {
                stability: state.stability,
                difficulty: state.difficulty,
            })
        };
        let now = UnixTime::now();
        let days_since_last_review = state.last_review_time.days_since(now);
        let next_states = self
            .0
            .next_states(memory_state, desired_retention, days_since_last_review)
            .unwrap();
        let new_state = if success {
            next_states.good
        } else {
            next_states.again
        };

        NextReviewState {
            stability: new_state.memory.stability,
            difficulty: new_state.memory.difficulty,
            due_time: now.add_days(new_state.interval.round().max(1.0)),
        }
    }
}

pub struct CurrentReviewState {
    pub stability: f32,
    pub difficulty: f32,
    pub last_review_time: UnixTime,
}

pub struct NextReviewState {
    pub stability: f32,
    pub difficulty: f32,
    pub due_time: UnixTime,
}
