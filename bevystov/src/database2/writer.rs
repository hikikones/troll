use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use crate::database2::{AudioExt, AudioFile, AudioFileError, AudioResult, TrackId, TrackRating};

type WriteResult = Result<WriteSuccess, WriteFailure>;
type WriteHandle = std::thread::JoinHandle<WriteResult>;

// TODO: Maybe make custom error for a message with path included.

pub struct RatingsWriter {
    queue: VecDeque<WriteRequest>,
    handle: Option<WriteHandle>,
}

impl RatingsWriter {
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            handle: None,
        }
    }

    pub fn push(&mut self, req: WriteRequest) {
        self.queue.push_back(req);
    }

    pub fn poll(&mut self) -> Option<WriteResult> {
        let Some(handle) = self.handle.as_ref() else {
            self.handle = self.queue.pop_front().map(start_write);
            return None;
        };

        if handle.is_finished()
            && let Some(handle) = self.handle.take()
            && let Ok(result) = handle.join()
        {
            return Some(result);
        }

        None
    }
}

pub struct WriteRequest {
    pub path: PathBuf,
    pub id: TrackId,
    pub rating: TrackRating,
}

pub struct WriteSuccess {
    pub id: TrackId,
    pub rating: TrackRating,
}

pub struct WriteFailure {
    pub path: PathBuf,
    pub error: AudioFileError,
}

fn start_write(request: WriteRequest) -> WriteHandle {
    let WriteRequest { path, id, rating } = request;

    std::thread::spawn(move || match write_rating(&path, rating) {
        Ok(_) => Ok(WriteSuccess { id, rating }),
        Err(error) => Err(WriteFailure { path, error }),
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
    // Make sure we finish writing
    fn drop(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };

        if let Err(err) = handle.join() {
            eprintln!("Thread for writing audio rating has panicked: {err:?}");
        }
    }
}
