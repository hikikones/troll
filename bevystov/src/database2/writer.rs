use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use crate::database2::{AudioExt, AudioFile, AudioFileError, AudioResult, TrackId, TrackRating};

type WriteResult = Result<(), WriteFailure>;
type WriteHandle = std::thread::JoinHandle<WriteResult>;

pub struct RatingsWriter {
    queue: VecDeque<WriteRequest>,
    state: Option<State>,
}

enum State {
    Request(WriteRequest),
    Handle(WriteHandle),
}

impl RatingsWriter {
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            state: None,
        }
    }

    pub fn push(&mut self, req: WriteRequest) {
        if self.state.is_none() {
            self.state = Some(State::Request(req));
        } else {
            self.queue.push_back(req);
        }
    }

    pub fn poll(&mut self, current_track_id: Option<TrackId>) -> Option<WriteResult> {
        let state = self.take_current_or_next()?;

        match state {
            State::Request(request) => {
                if current_track_id == Some(request.id) {
                    self.state = Some(State::Request(request));
                } else {
                    let handle = start_write(request);
                    self.state = Some(State::Handle(handle));
                }

                None
            }
            State::Handle(handle) => {
                if !handle.is_finished() {
                    self.state = Some(State::Handle(handle));
                    return None;
                }

                let Ok(result) = handle.join() else {
                    return None;
                };

                Some(result)
            }
        }
    }

    fn take_current_or_next(&mut self) -> Option<State> {
        self.state
            .take()
            .or_else(|| self.queue.pop_front().map(|req| State::Request(req)))
    }
}

pub struct WriteRequest {
    pub path: PathBuf,
    pub id: TrackId,
    pub old_rating: TrackRating,
    pub new_rating: TrackRating,
}

pub struct WriteFailure {
    pub id: TrackId,
    pub old_rating: TrackRating,
    pub path: PathBuf,
    pub error: AudioFileError,
}

fn start_write(request: WriteRequest) -> WriteHandle {
    let WriteRequest {
        path,
        id,
        old_rating,
        new_rating,
    } = request;

    std::thread::spawn(move || {
        write_rating(&path, new_rating).map_err(|error| WriteFailure {
            id,
            old_rating,
            path,
            error,
        })
    })
}

fn write_rating(path: &Path, rating: TrackRating) -> AudioResult<()> {
    let ext = AudioExt::from_path(path).unwrap();
    let mut audio_file = AudioFile::read_all(path, ext)?;
    let tag = audio_file.vorbis_comments_mut()?;

    match rating {
        TrackRating::None => {
            let _ = tag.remove("RATING");
        }
        _ => {
            tag.insert(
                String::from("RATING"),
                String::from(rating.into_vorbis_comments()),
            );
        }
    }
    audio_file.save(path)
}

impl Drop for RatingsWriter {
    // Make sure we finish writing if we have a current request
    fn drop(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };

        let handle = match state {
            State::Request(request) => start_write(request),
            State::Handle(handle) => handle,
        };

        if let Err(err) = handle.join() {
            eprintln!("Thread for writing audio rating has panicked: {err:?}");
        }
    }
}
