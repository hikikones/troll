use std::{borrow::Cow, fs::File, path::Path, range::Range, time::Duration};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile as LoftyAudioFile,
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, PopularimeterFrame},
    mpeg::MpegFile,
    ogg::{OggPictureStorage, OpusFile, VorbisFile, tag::VorbisComments},
    picture::PictureType,
};
use shared::sqlite::{
    FromSql, FromSqlError, FromSqlResult, SqliteId, SqliteResult, ToSql, ToSqlOutput, Value,
    ValueRef,
};
use utils::StringExt;

pub type AudioResult<T> = Result<T, AudioFileError>;

pub struct AudioFile(AudioFormat);

enum AudioFormat {
    Flac(FlacFile),
    Opus(OpusFile),
    Vorbis(VorbisFile),
    Mpeg(MpegFile),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AudioFileExt {
    #[default]
    Flac,
    Opus,
    Ogg,
    Mp3,
}

impl AudioFileExt {
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| {
            if ext.eq_ignore_ascii_case("flac") {
                Some(Self::Flac)
            } else if ext.eq_ignore_ascii_case("ogg") {
                Some(Self::Ogg)
            } else if ext.eq_ignore_ascii_case("mp3") {
                Some(Self::Mp3)
            } else {
                #[cfg(feature = "opus")]
                if ext.eq_ignore_ascii_case("opus") {
                    return Some(Self::Opus);
                }
                None
            }
        })
    }

    pub fn try_from_path(path: &Path) -> AudioResult<Self> {
        Self::from_path(path).ok_or(AudioFileError::InvalidExtension)
    }

    pub const fn as_lower_case(self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::Ogg => "ogg",
            Self::Mp3 => "mp3",
        }
    }

    pub const fn as_upper_case(self) -> &'static str {
        match self {
            Self::Flac => "FLAC",
            Self::Opus => "OPUS",
            Self::Ogg => "OGG",
            Self::Mp3 => "MP3",
        }
    }
}

impl AudioFile {
    pub fn read_all(path: impl AsRef<Path>, ext: AudioFileExt) -> AudioResult<Self> {
        Self::read(path, ext, ParseOptions::new())
    }

    pub fn read_metadata(path: impl AsRef<Path>, ext: AudioFileExt) -> AudioResult<Self> {
        Self::read(
            path,
            ext,
            ParseOptions::new()
                .read_tags(true)
                .read_properties(true)
                .read_cover_art(false),
        )
    }

    pub fn read_cover(path: impl AsRef<Path>, ext: AudioFileExt) -> AudioResult<Self> {
        Self::read(
            path,
            ext,
            ParseOptions::new()
                .read_cover_art(true)
                .read_tags(ext != AudioFileExt::Flac)
                .read_properties(false),
        )
    }

    fn read(path: impl AsRef<Path>, ext: AudioFileExt, options: ParseOptions) -> AudioResult<Self> {
        let mut file = File::open(path.as_ref())?;

        let audio_format = match ext {
            AudioFileExt::Flac => {
                let flac = FlacFile::read_from(&mut file, options)?;
                AudioFormat::Flac(flac)
            }
            AudioFileExt::Opus => {
                let opus = OpusFile::read_from(&mut file, options)?;
                AudioFormat::Opus(opus)
            }
            AudioFileExt::Ogg => {
                let ogg_vorbis = VorbisFile::read_from(&mut file, options)?;
                AudioFormat::Vorbis(ogg_vorbis)
            }
            AudioFileExt::Mp3 => {
                let mpeg = MpegFile::read_from(&mut file, options)?;
                AudioFormat::Mpeg(mpeg)
            }
        };

        Ok(Self(audio_format))
    }

    pub fn metadata(&self) -> AudioResult<AudioMetadata> {
        let mut metadata = AudioMetadata::default();
        self.metadata_in_place(&mut metadata)?;
        Ok(metadata)
    }

    pub fn metadata_in_place(&self, metadata: &mut AudioMetadata) -> AudioResult<()> {
        let vorbis = match &self.0 {
            AudioFormat::Flac(flac) => flac
                .vorbis_comments()
                .ok_or(AudioFileError::MissingVorbisComments),
            AudioFormat::Opus(opus) => Ok(opus.vorbis_comments()),
            AudioFormat::Vorbis(vorbis) => Ok(vorbis.vorbis_comments()),
            AudioFormat::Mpeg(mpeg) => {
                let id3v2 = mpeg.id3v2().ok_or(AudioFileError::MissingId3v2Tag)?;
                metadata_id3v2(id3v2, metadata);
                return Ok(());
            }
        }?;
        metadata_vorbis(vorbis, metadata);
        Ok(())
    }

    pub fn properties(&self) -> AudioProperties {
        match &self.0 {
            AudioFormat::Flac(flac) => {
                let p = flac.properties();
                AudioProperties {
                    duration: p.duration().into(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: Some(p.bit_depth()),
                    sample_rate: Some(p.sample_rate()),
                }
            }
            AudioFormat::Opus(opus) => {
                let p = opus.properties();
                AudioProperties {
                    duration: p.duration().into(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate: None,
                }
            }
            AudioFormat::Vorbis(vorbis) => {
                let p = vorbis.properties();
                AudioProperties {
                    duration: p.duration().into(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate: Some(p.sample_rate()),
                }
            }
            AudioFormat::Mpeg(mpeg) => {
                let p = mpeg.properties();
                AudioProperties {
                    duration: p.duration().into(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate: Some(p.sample_rate()),
                }
            }
        }
    }

    pub fn front_cover(&self) -> Option<&[u8]> {
        let pictures = match &self.0 {
            AudioFormat::Flac(flac) => flac.pictures(),
            AudioFormat::Opus(opus) => opus.vorbis_comments().pictures(),
            AudioFormat::Vorbis(vorbis) => vorbis.vorbis_comments().pictures(),
            AudioFormat::Mpeg(mpeg) => {
                return mpeg
                    .id3v2()?
                    .iter()
                    .filter_map(|frame| match frame {
                        Frame::Picture(picture) => Some(&picture.picture),
                        _ => None,
                    })
                    .find(|picture| picture.pic_type() == PictureType::CoverFront)
                    .map(|picture| picture.data());
            }
        };

        pictures
            .iter()
            .find(|(picture, _)| picture.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures.first())
            .map(|(picture, _)| picture.data())
    }

    pub fn set_rating(&mut self, rating: TrackRating) -> AudioResult<()> {
        let vorbis = match &mut self.0 {
            AudioFormat::Flac(flac) => flac
                .vorbis_comments_mut()
                .ok_or(AudioFileError::MissingVorbisComments),
            AudioFormat::Opus(opus) => Ok(opus.vorbis_comments_mut()),
            AudioFormat::Vorbis(vorbis) => Ok(vorbis.vorbis_comments_mut()),
            AudioFormat::Mpeg(mpeg) => {
                let id3v2 = mpeg.id3v2_mut().ok_or(AudioFileError::MissingId3v2Tag)?;
                match rating {
                    TrackRating::None => {
                        const POPM_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("POPM"));
                        let _ = id3v2.remove(&POPM_ID);
                    }
                    _ => {
                        id3v2.insert(Frame::Popularimeter(PopularimeterFrame::new(
                            String::new(),
                            rating.into_id3v2(),
                            0,
                        )));
                    }
                }
                return Ok(());
            }
        }?;

        match rating {
            TrackRating::None => {
                let _ = vorbis.remove("RATING");
            }
            _ => {
                vorbis.insert(
                    String::from("RATING"),
                    String::from(rating.into_vorbis_comments()),
                );
            }
        }

        Ok(())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> AudioResult<()> {
        let options = WriteOptions::new();

        match &self.0 {
            AudioFormat::Flac(flac) => {
                flac.save_to_path(path, options)?;
            }
            AudioFormat::Opus(opus) => {
                opus.save_to_path(path, options)?;
            }
            AudioFormat::Vorbis(vorbis) => {
                vorbis.save_to_path(path, options)?;
            }
            AudioFormat::Mpeg(mpeg) => {
                mpeg.save_to_path(path, options)?;
            }
        }

        Ok(())
    }
}

fn metadata_vorbis(vorbis: &VorbisComments, data: &mut AudioMetadata) {
    #[derive(Clone, Copy)]
    enum Key {
        Title,
        Artist,
        Album,
        AlbumArtist,
        Rating,
        TrackNumber,
        Date,
    }

    impl Key {
        const fn from_str(key: &str) -> Option<Self> {
            if key.eq_ignore_ascii_case("TITLE") {
                Some(Self::Title)
            } else if key.eq_ignore_ascii_case("ARTIST") {
                Some(Self::Artist)
            } else if key.eq_ignore_ascii_case("ALBUM") {
                Some(Self::Album)
            } else if key.eq_ignore_ascii_case("ALBUMARTIST") {
                Some(Self::AlbumArtist)
            } else if key.eq_ignore_ascii_case("RATING") {
                Some(Self::Rating)
            } else if key.eq_ignore_ascii_case("TRACKNUMBER")
                || key.eq_ignore_ascii_case("TRACKNUM")
            {
                Some(Self::TrackNumber)
            } else if key.eq_ignore_ascii_case("DATE") {
                Some(Self::Date)
            } else {
                None
            }
        }
    }

    for (key, val) in vorbis
        .items()
        .filter_map(|(key, val)| Key::from_str(key).map(|key| (key, val)))
    {
        match key {
            Key::Title => data.title = data.text.push_str_get_range(val),
            Key::Artist => {
                let range = data.text.push_str_get_range(val);
                data.artists.push(range);
            }
            Key::Album => data.album = data.text.push_str_get_range(val),
            Key::AlbumArtist => data.album_artist = data.text.push_str_get_range(val),
            Key::Rating => data.rating = TrackRating::from_vorbis_comments(val),
            Key::TrackNumber => data.number = val.parse::<u8>().unwrap_or_default(),
            Key::Date => data.date = data.text.push_str_get_range(val),
        }
    }
}

fn metadata_id3v2(id3v2: &Id3v2Tag, data: &mut AudioMetadata) {
    const TITLE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIT2"));
    const ARTISTS_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TPE1"));
    const ALBUM_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TALB"));
    const ALBUM_ARTIST_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TPE2"));
    const RATING_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("POPM"));
    const TRACK_NUMBER_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TRCK"));
    const DATE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TDRC"));

    if let Some(title) = id3v2.get_text(&TITLE_ID) {
        data.title = data.text.push_str_get_range(title);
    }

    if let Some(artists) = id3v2.get_texts(&ARTISTS_ID) {
        for artist in artists {
            let range = data.text.push_str_get_range(artist);
            data.artists.push(range);
        }
    }

    if let Some(album) = id3v2.get_text(&ALBUM_ID) {
        data.album = data.text.push_str_get_range(album);
    }

    if let Some(album_artist) = id3v2.get_text(&ALBUM_ARTIST_ID) {
        data.album_artist = data.text.push_str_get_range(album_artist);
    }

    if let Some(date) = id3v2.get_text(&DATE_ID) {
        data.date = data.text.push_str_get_range(date);
    }

    data.rating = id3v2
        .get(&RATING_ID)
        .map(|frame| match frame {
            Frame::Popularimeter(popm) => TrackRating::from_id3v2(popm.rating),
            _ => TrackRating::None,
        })
        .unwrap_or_default();

    data.number = id3v2
        .get_text(&TRACK_NUMBER_ID)
        .map(|num| num.parse().unwrap_or_default())
        .unwrap_or_default();
}

#[derive(Default, Clone)]
pub struct AudioMetadata {
    pub text: String,
    pub artists: Vec<Range<usize>>,
    pub number: u8,
    pub rating: TrackRating,
    pub title: Range<usize>,
    pub album: Range<usize>,
    pub album_artist: Range<usize>,
    pub date: Range<usize>,
}

impl AudioMetadata {
    pub fn slice(&self, range: Range<usize>) -> &str {
        &self.text[range]
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.artists.clear();
        self.number = 0;
        self.rating = TrackRating::default();
        self.title = Range::default();
        self.album = Range::default();
        self.album_artist = Range::default();
        self.date = Range::default();
    }
}

#[derive(Default, Clone)]
pub struct AudioProperties {
    pub duration: TrackDuration,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate: Option<u32>,
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
)]
#[repr(u8)]
pub enum TrackRating {
    #[default]
    None = 0,
    Awful = 1,
    Bad = 2,
    Ok = 3,
    Good = 4,
    Amazing = 5,
}

impl TrackRating {
    fn from_vorbis_comments(value: &str) -> Self {
        value
            .parse::<u8>()
            .map(|num| match num {
                0 => Self::None,
                ..=20 => Self::Awful,
                ..=40 => Self::Bad,
                ..=60 => Self::Ok,
                ..=80 => Self::Good,
                ..=255 => Self::Amazing,
            })
            .unwrap_or_default()
    }

    const fn into_vorbis_comments(self) -> &'static str {
        match self {
            Self::None => "0",
            Self::Awful => "20",
            Self::Bad => "40",
            Self::Ok => "60",
            Self::Good => "80",
            Self::Amazing => "100",
        }
    }

    const fn from_id3v2(value: u8) -> Self {
        match value {
            0 => Self::None,
            ..=51 => Self::Awful,
            ..=102 => Self::Bad,
            ..=153 => Self::Ok,
            ..=204 => Self::Good,
            ..=255 => Self::Amazing,
        }
    }

    const fn into_id3v2(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Awful => 51,
            Self::Bad => 102,
            Self::Ok => 153,
            Self::Good => 204,
            Self::Amazing => 255,
        }
    }

    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::None),
            '1' => Some(Self::Awful),
            '2' => Some(Self::Bad),
            '3' => Some(Self::Ok),
            '4' => Some(Self::Good),
            '5' => Some(Self::Amazing),
            _ => None,
        }
    }

    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn stars(self) -> &'static str {
        use shared::symbols::{STAR, repeat};
        match self {
            Self::None => "",
            Self::Awful => STAR,
            Self::Bad => repeat!(STAR, 2),
            Self::Ok => repeat!(STAR, 3),
            Self::Good => repeat!(STAR, 4),
            Self::Amazing => repeat!(STAR, 5),
        }
    }

    pub const fn stars_split(self) -> (&'static str, &'static str) {
        use shared::symbols::{STAR_BIG, repeat};
        match self {
            Self::None => (repeat!(STAR_BIG, 0), repeat!(STAR_BIG, 5)),
            Self::Awful => (repeat!(STAR_BIG, 1), repeat!(STAR_BIG, 4)),
            Self::Bad => (repeat!(STAR_BIG, 2), repeat!(STAR_BIG, 3)),
            Self::Ok => (repeat!(STAR_BIG, 3), repeat!(STAR_BIG, 2)),
            Self::Good => (repeat!(STAR_BIG, 4), repeat!(STAR_BIG, 1)),
            Self::Amazing => (repeat!(STAR_BIG, 5), repeat!(STAR_BIG, 0)),
        }
    }
}

impl ToSql for TrackRating {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(*self as SqliteId)))
    }
}

impl FromSql for TrackRating {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().and_then(|n| match n {
            0 => Ok(Self::None),
            1 => Ok(Self::Awful),
            2 => Ok(Self::Bad),
            3 => Ok(Self::Ok),
            4 => Ok(Self::Good),
            5 => Ok(Self::Amazing),
            _ => Err(FromSqlError::Other(
                format!("Audio rating {n} is out of range (0-5)").into(),
            )),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TrackDuration(u16);

impl TrackDuration {
    pub const fn new(seconds: u16) -> Self {
        Self(seconds)
    }

    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration.as_secs() as u16)
    }

    pub const fn progress(self, other: Self) -> f32 {
        if self.0 >= other.0 || other.0 == 0 {
            return 1.0;
        }

        self.as_f32() / other.as_f32()
    }

    pub const fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }

    pub const fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub const fn mul_f32(self, v: f32) -> Self {
        Self((self.0 as f32 * v) as u16)
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 < other.0 { other } else { self }
    }

    pub const fn as_f32(self) -> f32 {
        self.0 as f32
    }

    pub const fn as_duration(self) -> Duration {
        Duration::from_secs(self.0 as u64)
    }
}

impl From<Duration> for TrackDuration {
    fn from(value: Duration) -> Self {
        Self::from_duration(value)
    }
}

impl From<TrackDuration> for Duration {
    fn from(value: TrackDuration) -> Self {
        value.as_duration()
    }
}

impl std::fmt::Display for TrackDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = self.0 % 60;
        let minutes = (self.0 - seconds) / 60;
        f.write_fmt(format_args!("{:02}:{:02}", minutes, seconds))
    }
}

impl ToSql for TrackDuration {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0 as SqliteId)))
    }
}

impl FromSql for TrackDuration {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n as u16))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TrackDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl TrackDate {
    /// Parses dates in ISO 8601 "YYYY-MM-DD".
    /// Months and days defaults to zero when not found.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('-');

        let year = parts.next()?.parse().ok()?;
        let month = parts
            .next()
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();
        let day = parts
            .next()
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();

        Some(Self { year, month, day })
    }

    pub const fn from_u32(yyyymmdd: u32) -> Self {
        Self {
            year: (yyyymmdd / 10_000) as u16,
            month: ((yyyymmdd / 100) % 100) as u8,
            day: (yyyymmdd % 100) as u8,
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.year as u32 * 10_000 + self.month as u32 * 100 + self.day as u32
    }
}

impl std::fmt::Display for TrackDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        ))
    }
}

impl ToSql for TrackDate {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(
            Value::Integer(self.as_u32() as SqliteId),
        ))
    }
}

impl FromSql for TrackDate {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self::from_u32(n as u32))
    }
}

#[derive(Debug)]
pub enum AudioFileError {
    Io(std::io::Error),
    Parse(lofty::error::FileParseError),
    Encode(lofty::error::FileEncodingError),
    MissingVorbisComments,
    MissingId3v2Tag,
    InvalidExtension,
}

impl std::fmt::Display for AudioFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFileError::Io(err) => err.fmt(f),
            AudioFileError::Parse(err) => err.fmt(f),
            AudioFileError::Encode(err) => err.fmt(f),
            AudioFileError::MissingVorbisComments => {
                f.write_str("No VorbisComments metadata container found")
            }
            AudioFileError::MissingId3v2Tag => f.write_str("No ID3v2 tag found"),
            AudioFileError::InvalidExtension => f.write_str("No valid audio file extension found"),
        }
    }
}

impl std::error::Error for AudioFileError {}

impl From<std::io::Error> for AudioFileError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<lofty::error::FileParseError> for AudioFileError {
    fn from(err: lofty::error::FileParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<lofty::error::FileEncodingError> for AudioFileError {
    fn from(err: lofty::error::FileEncodingError) -> Self {
        Self::Encode(err)
    }
}
