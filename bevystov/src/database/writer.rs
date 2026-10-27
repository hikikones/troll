use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::database::{AudioExt, AudioFile, AudioFileError, AudioResult, TrackId, TrackRating};

type WriteResult = Result<(), WriteFailure>;
type WriteHandle = std::thread::JoinHandle<WriteResult>;

pub struct RatingsWriter {
    queue: HashMap<TrackId, WriteData>,
    handle: Option<WriteHandle>,
    current_track_id: Option<TrackId>,
}

impl RatingsWriter {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
            handle: None,
            current_track_id: None,
        }
    }

    pub fn push(&mut self, req: WriteRequest) {
        let should_remove = {
            let data = self
                .queue
                .entry(req.id)
                .and_modify(|data| {
                    data.new_rating = req.new_rating;
                })
                .or_insert(WriteData {
                    old_rating: req.old_rating,
                    new_rating: req.new_rating,
                    path: req.path,
                });

            data.old_rating == data.new_rating
        };

        if should_remove {
            self.queue.remove(&req.id);
        }
    }

    pub fn poll(&mut self, current_track_id: Option<TrackId>) -> Option<WriteResult> {
        self.current_track_id = current_track_id;

        let handle = self.handle.take().or_else(|| self.next())?;

        if !handle.is_finished() {
            self.handle = Some(handle);
            return None;
        }

        let Ok(result) = handle.join() else {
            return None;
        };

        Some(result)
    }

    fn next(&mut self) -> Option<WriteHandle> {
        let next = self
            .queue
            .keys()
            .copied()
            .find(|id| self.current_track_id != Some(*id))?;

        let data = self.queue.remove(&next).unwrap();
        let handle = start_write(next, data);
        Some(handle)
    }
}

pub struct WriteRequest {
    pub id: TrackId,
    pub old_rating: TrackRating,
    pub new_rating: TrackRating,
    pub path: PathBuf,
}

pub struct WriteFailure {
    pub id: TrackId,
    pub old_rating: TrackRating,
    pub path: PathBuf,
    pub error: AudioFileError,
}

struct WriteData {
    old_rating: TrackRating,
    new_rating: TrackRating,
    path: PathBuf,
}

fn start_write(id: TrackId, data: WriteData) -> WriteHandle {
    let WriteData {
        old_rating,
        new_rating,
        path,
    } = data;

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
    let ext = AudioExt::try_from_path(path)?;
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
    fn drop(&mut self) {
        // Make sure we finish writing if we have a handle
        if let Some(handle) = self.handle.take() {
            if let Err(err) = handle.join() {
                eprintln!("Thread for writing audio rating has panicked: {err:?}");
            }
        };

        // Also make sure to write rating for current track id
        let Some(id) = self.current_track_id else {
            return;
        };

        let Some(data) = self.queue.remove(&id) else {
            return;
        };

        let handle = start_write(id, data);
        if let Err(err) = handle.join() {
            eprintln!("Thread for writing audio rating has panicked: {err:?}");
        }

        // Ignore the rest of the queue
    }
}
