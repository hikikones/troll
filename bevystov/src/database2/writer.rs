use std::{collections::VecDeque, path::PathBuf};

use crate::database2::{AudioExt, AudioFile, AudioFileError, TrackId, TrackRating};

type WriteResult = Result<WriteRequest, AudioFileError>;
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

    pub fn update(&mut self, mut on_result: impl FnMut(WriteResult)) {
        let Some(handle) = self.handle.as_ref() else {
            self.handle = self.queue.pop_front().map(start_write);
            return;
        };

        if handle.is_finished()
            && let Some(handle) = self.handle.take()
            && let Ok(result) = handle.join()
        {
            on_result(result);
        }
    }
}

pub struct WriteRequest {
    pub path: PathBuf,
    pub id: TrackId,
    pub rating: TrackRating,
}

fn start_write(request: WriteRequest) -> WriteHandle {
    let ext = AudioExt::from_path(&request.path).unwrap();
    std::thread::spawn(move || {
        let mut audio_file = AudioFile::read(&request.path, ext)?;
        let tag = audio_file.vorbis_comments_mut()?;

        match request.rating {
            TrackRating::None => {
                let _ = tag.remove("RATING");
            }
            _ => {
                tag.insert(
                    String::from("RATING"),
                    String::from(request.rating.into_vorbis_comments()),
                );
            }
        }
        audio_file.save(&request.path)?;

        Ok(request)
    })
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
