use std::{
    collections::HashSet,
    ops::Range,
    path::{Path, PathBuf},
};

mod decoder;
mod player;
mod queue;

use decoder::*;
use player::*;
use queue::*;
use utils::StringExt;

use crate::database2::{AudioExt, Database, TrackDuration, TrackId, TrackRating};

pub type JukeboxResult<T> = Result<T, JukeboxError>;

pub struct Jukebox {
    current: Option<(TrackId, usize)>,
    queue: PlayQueue,
    state: PlayState,
    decoder: AudioDecoder,
    events: Vec<JukeboxEvent>,
    faulty: HashSet<TrackId>,
    player: AudioPlayer,
    metadata: Metadata,
}

#[derive(Debug, Default)]
struct Metadata {
    text: String,
    path: PathBuf,
    ext: AudioExt,
    rating: TrackRating,
    duration: TrackDuration,
    bit_rate_kbps: u32,
    bit_depth: Option<u8>,
    sample_rate: Option<u32>,
    title: Range<usize>,
    artists: Range<usize>,
    album: Range<usize>,
}

impl Metadata {
    fn update(&mut self, id: TrackId, db: &Database) {
        self.text.clear();
        self.path.clear();

        db.track_info(id, |info| {
            self.path.push(info.path);
            self.ext = AudioExt::from_path(info.path).unwrap_or_default();
            self.rating = info.rating;
            self.duration = info.duration;
            self.bit_rate_kbps = info.bit_rate_kbps;
            self.bit_depth = info.bit_depth;
            self.sample_rate = info.sample_rate;
            self.title = self.text.push_str_get_range(info.title);

            db.album_name(info.album_id, |album| {
                self.album = self.text.push_str_get_range(album)
            });
        });

        let start = self.text.len();
        db.track_artists(id, |artist| {
            self.text.push_str(artist);
            self.text.push(' ');
        });
        self.text.pop();
        self.artists = start..self.text.len();
    }
}

pub struct CurrentTrack<'a> {
    id: TrackId,
    index: usize,
    data: &'a Metadata,
}

impl<'a> CurrentTrack<'a> {
    pub const fn id(&self) -> TrackId {
        self.id
    }

    pub const fn queue_index(&self) -> usize {
        self.index
    }

    pub const fn rating(&self) -> TrackRating {
        self.data.rating
    }

    pub const fn duration(&self) -> TrackDuration {
        self.data.duration
    }

    pub const fn bit_rate_kbps(&self) -> u32 {
        self.data.bit_rate_kbps
    }

    pub const fn bit_depth(&self) -> Option<u8> {
        self.data.bit_depth
    }

    pub const fn sample_rate(&self) -> Option<u32> {
        self.data.sample_rate
    }

    pub fn title(&self) -> &str {
        &self.data.text[self.data.title.clone()]
    }

    pub fn artists(&self) -> &str {
        &self.data.text[self.data.artists.clone()]
    }

    pub fn album(&self) -> &str {
        &self.data.text[self.data.album.clone()]
    }

    pub fn path(&self) -> &Path {
        &self.data.path
    }

    pub const fn ext(&self) -> AudioExt {
        self.data.ext
    }
}

pub enum JukeboxEvent {
    Play(Option<TrackId>),
    Pause,
    Stop,
    Error(JukeboxError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Track,
}

impl Jukebox {
    pub fn new() -> JukeboxResult<Self> {
        let player = AudioPlayer::new().map_err(|err| JukeboxError {
            message: String::from("Failed to create audio player"),
            source: Some(JukeboxErrorSource::Player(err)),
        })?;

        Ok(Self {
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            decoder: AudioDecoder::new(),
            events: Vec::new(),
            faulty: HashSet::new(),
            player,
            metadata: Metadata::default(),
        })
    }

    pub const fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn is_faulty(&self, id: TrackId) -> bool {
        self.faulty.contains(&id)
    }

    pub const fn len(&self) -> usize {
        self.queue.len()
    }

    pub const fn queue(&self) -> usize {
        if self.queue.is_empty() {
            return 0;
        }

        let len = self.queue.queue_len();
        if self.current.is_none() { len + 1 } else { len }
    }

    pub const fn history(&self) -> usize {
        self.queue.history_len()
    }

    pub const fn has_current(&self) -> bool {
        self.current.is_some()
    }

    pub fn track(&self) -> Option<CurrentTrack<'_>> {
        self.current.map(|(id, index)| CurrentTrack {
            id,
            index,
            data: &self.metadata,
        })
    }

    pub fn track_id(&self) -> Option<TrackId> {
        self.current.map(|(id, _)| id)
    }

    pub fn queue_index(&self) -> Option<usize> {
        self.current.map(|(_, index)| index)
    }

    pub fn track_pos(&self) -> TrackDuration {
        TrackDuration::from_duration(self.player.position())
    }

    pub fn get(&self, index: usize) -> Option<TrackId> {
        self.queue.get(index)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, usize)> {
        self.queue.iter()
    }

    pub fn shuffle(&mut self) -> bool {
        let start_index = self
            .queue_index()
            .map(|i| i + 1)
            .or(self.queue.index())
            .unwrap_or(0);
        self.queue.shuffle(start_index)
    }

    pub fn move_up(&mut self, i: usize) -> bool {
        let success = self.queue.move_up(i);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_up_range(&mut self, start: usize, end: usize) -> bool {
        let success = self.queue.move_up_range(start, end);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_down(&mut self, i: usize) -> bool {
        let success = self.queue.move_down(i);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_down_range(&mut self, start: usize, end: usize) -> bool {
        let success = self.queue.move_down_range(start, end);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn remove(&mut self, index: usize) -> bool {
        let keep_current = self.current.is_some();
        let removal = self.queue.remove(index, keep_current).is_some();

        if keep_current {
            self.current = self.queue.current();
        }

        removal
    }

    pub fn remove_range(&mut self, start: usize, end: usize) -> bool {
        let keep_current = self.current.is_some();
        let removal = self.queue.remove_range(start, end, keep_current);

        if keep_current {
            self.current = self.queue.current();
        }

        removal
    }

    pub fn clear(&mut self) -> bool {
        if self.queue.is_empty() || (self.queue.len() == 1 && self.queue.current() == self.current)
        {
            return false;
        }

        self.queue.clear();

        if let Some((id, _)) = self.current.take() {
            self.current = self.queue.add(id).next();
        }

        true
    }

    pub fn add(&mut self, id: TrackId) {
        self.queue.add(id);
    }

    pub fn add_next(&mut self, id: TrackId) {
        self.queue.add_next(id);
    }

    pub fn extend(&mut self, ids: impl IntoIterator<Item = TrackId>) {
        self.queue.extend(ids);
    }

    pub fn volume(&self) -> f32 {
        self.player.volume()
    }

    pub fn set_volume(&mut self, value: f32) {
        self.player.set_volume(value);
    }

    pub fn play_index(&mut self, index: usize, db: &Database) {
        if let Some(id) = self.queue.set_index(index) {
            self.state = PlayState::Track;
            self.start_decode(id, index, db);
        }
    }

    pub fn play_id(&mut self, id: TrackId, db: &Database) {
        let (id, index) = self.queue.add_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_decode(id, index, db);
    }

    pub fn play(&mut self) {
        if self.state == PlayState::Play {
            return;
        }

        self.state = PlayState::Play;
        self.player.play();
        self.events.push(JukeboxEvent::Play(None));
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Pause {
            return;
        }

        self.state = PlayState::Pause;
        self.player.pause();
        self.events.push(JukeboxEvent::Pause);
    }

    pub fn pause_or_play(&mut self) {
        match self.state {
            PlayState::Pause | PlayState::Stop => self.play(),
            _ => self.pause(),
        }
    }

    pub fn stop(&mut self) {
        if self.state == PlayState::Stop {
            return;
        }

        self.state = PlayState::Stop;
        self.current = None;
        self.decoder.clear();
        self.player.clear();
        self.events.push(JukeboxEvent::Stop);
    }

    pub fn play_next(&mut self, db: &Database, skip: TrackRating) {
        if db.is_tracks_empty() {
            return;
        }

        let mut next = self.queue.current_or_next();
        if next == self.current {
            next = self.queue.next();
        }

        loop {
            match next {
                Some((id, index)) => {
                    if self.is_faulty(id) || self.should_skip(id, db, skip) {
                        next = self.queue.next();
                        continue;
                    }

                    self.state = PlayState::Next;
                    self.start_decode(id, index, db);
                    return;
                }
                None => {
                    break;
                }
            }
        }

        // No tracks in the queue, play a random next
        const TRIES: u8 = 4;
        let mut count = 0;
        let mut rand;

        match self.track_id() {
            Some(id) => {
                loop {
                    rand = match db.random_track_id_except(id) {
                        Some(id) => id,
                        None => {
                            // Only one track in the entire db
                            return;
                        }
                    };
                    count += 1;

                    let is_bad = self.is_faulty(rand) || self.should_skip(rand, db, skip);
                    if !is_bad || count >= TRIES {
                        break;
                    }
                }
            }
            None => loop {
                rand = db.random_track_id();
                count += 1;

                let is_bad = self.is_faulty(rand) || self.should_skip(rand, db, skip);
                if !is_bad || count >= TRIES {
                    break;
                }
            },
        }

        let (id, index) = self.queue.add(rand).next().unwrap();
        self.state = PlayState::Next;
        self.start_decode(id, index, db);
    }

    pub fn play_previous(&mut self, db: &Database, skip: TrackRating) {
        while let Some((id, index)) = self.queue.previous() {
            if self.is_faulty(id) || self.should_skip(id, db, skip) {
                continue;
            }

            self.state = PlayState::Previous;
            self.start_decode(id, index, db);
            return;
        }

        // No valid previous found, update queue index
        self.sync_queue_index();
    }

    pub fn seek_forwards(&mut self) -> bool {
        if self.current.is_none() || self.player.is_paused() || self.player.is_empty() {
            return false;
        }

        let pos = self.track_pos();
        let min = TrackDuration::new(1);
        let secs = self.metadata.duration.mul_f32(0.10).max(min);
        self.player.seek(pos.add(secs).as_duration());
        true
    }

    pub fn seek_backwards(&mut self) -> bool {
        if self.current.is_none() || self.player.is_paused() || self.player.is_empty() {
            return false;
        }

        let pos = self.track_pos();
        let min = TrackDuration::new(2);
        let secs = self.metadata.duration.mul_f32(0.10).max(min);
        self.player.seek(pos.sub(secs).as_duration());
        true
    }

    fn should_skip(&self, id: TrackId, db: &Database, skip: TrackRating) -> bool {
        let rating = db.track_rating(id);
        match rating {
            TrackRating::None => false,
            _ => rating.as_u8() <= skip.as_u8(),
        }
    }

    fn start_decode(&mut self, id: TrackId, index: usize, db: &Database) {
        let path = db.track_path(id);
        self.decoder.start(DecodeRequest { id, index, path });
    }

    fn sync_queue_index(&mut self) {
        match self.current {
            Some((_, index)) => {
                self.queue.set_index(index);
            }
            None => {
                self.queue.reset();
            }
        }
    }

    pub fn update(
        &mut self,
        db: &Database,
        skip: TrackRating,
        mut on_event: impl FnMut(&Self, JukeboxEvent),
    ) {
        self.poll_decoder(db, skip);
        self.update_state(db, skip);

        // Drain events
        self.events.reverse();
        while let Some(event) = self.events.pop() {
            on_event(self, event);
        }
    }

    fn poll_decoder(&mut self, db: &Database, skip: TrackRating) {
        let Some(result) = self.decoder.poll() else {
            return;
        };

        match result {
            Ok(DecodeSuccess { id, index, decode }) => {
                self.player.clear();
                self.player.append(decode);
                if self.state != PlayState::Pause {
                    self.state = PlayState::Play;
                    self.player.play();
                }
                self.current = Some((id, index));
                self.metadata.update(id, db);
                self.events.push(JukeboxEvent::Play(Some(id)));
            }
            Err(DecodeFailure { id, path, error }) => {
                self.faulty.insert(id);
                self.events.push(JukeboxEvent::Error(JukeboxError {
                    message: format!("Failed to decode audio file '{}'", path.display()),
                    source: Some(JukeboxErrorSource::Decode(error)),
                }));

                match self.state {
                    PlayState::Play | PlayState::Next => {
                        self.play_next(db, skip);
                    }
                    PlayState::Previous => {
                        self.play_previous(db, skip);
                    }
                    PlayState::Track => {
                        self.state = PlayState::Play;
                        self.sync_queue_index();
                    }
                    _ => {}
                }
            }
        }
    }

    fn update_state(&mut self, db: &Database, skip: TrackRating) {
        // Play next when empty and idle
        if self.player.is_empty() && !self.player.is_paused() {
            match self.state {
                PlayState::Play => {
                    self.play_next(db, skip);
                }
                PlayState::Next | PlayState::Previous | PlayState::Track => {
                    self.state = PlayState::Play;
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
pub struct JukeboxError {
    message: String,
    source: Option<JukeboxErrorSource>,
}

#[derive(Debug)]
enum JukeboxErrorSource {
    Player(rodio::DeviceSinkError),
    Decode(DecodeError),
}

impl std::fmt::Display for JukeboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JukeboxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| match source {
            JukeboxErrorSource::Player(err) => err as _,
            JukeboxErrorSource::Decode(err) => err as _,
        })
    }
}
