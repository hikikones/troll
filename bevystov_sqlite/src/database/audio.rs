use std::{borrow::Cow, fs::File, path::Path, range::Range};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile as LoftyAudioFile,
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, PopularimeterFrame},
    mpeg::MpegFile,
    ogg::{OggPictureStorage, OpusFile, VorbisFile, tag::VorbisComments},
    picture::PictureType,
};
use utils::StringExt;

use super::{TrackDuration, TrackRating};

pub type AudioResult<T> = Result<T, AudioFileError>;

pub struct AudioFile(AudioFormat);

enum AudioFormat {
    Flac(FlacFile),
    Opus(OpusFile),
    Vorbis(VorbisFile),
    Mpeg(MpegFile),
}

impl AudioFile {
    pub fn read_all(path: impl AsRef<Path>, ext: AudioExt) -> AudioResult<Self> {
        Self::read(path, ext, ParseOptions::new())
    }

    pub fn read_metadata(path: impl AsRef<Path>, ext: AudioExt) -> AudioResult<Self> {
        Self::read(
            path,
            ext,
            ParseOptions::new()
                .read_tags(true)
                .read_properties(true)
                .read_cover_art(false),
        )
    }

    pub fn read_cover(path: impl AsRef<Path>, ext: AudioExt) -> AudioResult<Self> {
        Self::read(
            path,
            ext,
            ParseOptions::new()
                .read_cover_art(true)
                .read_tags(ext != AudioExt::Flac)
                .read_properties(false),
        )
    }

    fn read(path: impl AsRef<Path>, ext: AudioExt, options: ParseOptions) -> AudioResult<Self> {
        let mut file = File::open(path.as_ref())?;

        let audio_format = match ext {
            AudioExt::Flac => {
                let flac = FlacFile::read_from(&mut file, options)?;
                AudioFormat::Flac(flac)
            }
            AudioExt::Opus => {
                let opus = OpusFile::read_from(&mut file, options)?;
                AudioFormat::Opus(opus)
            }
            AudioExt::Ogg => {
                let ogg_vorbis = VorbisFile::read_from(&mut file, options)?;
                AudioFormat::Vorbis(ogg_vorbis)
            }
            AudioExt::Mp3 => {
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
                let id3 = mpeg.id3v2().ok_or(AudioFileError::MissingId3v2Tag)?;
                metadata_id3v2(id3, metadata);
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
                // let id3v2 = mpeg.id3v2()?;

                // let pictures = || {
                //     id3v2.into_iter().filter_map(|frame| {
                //         if let Frame::Picture(picture) = frame {
                //             Some(&picture.picture)
                //         } else {
                //             None
                //         }
                //     })
                // };

                // return pictures()
                //     .find(|p| p.pic_type() == PictureType::CoverFront)
                //     .or_else(|| pictures().next())
                //     .map(|p| p.data());
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
                const POPM_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("POPM"));
                let id3v2 = mpeg.id3v2_mut().ok_or(AudioFileError::MissingId3v2Tag)?;
                match rating {
                    TrackRating::None => {
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
    for (key, val) in vorbis.items() {
        match key {
            "TITLE" => data.title = data.text.push_str_get_range(val),
            "ARTIST" => {
                let range = data.text.push_str_get_range(val);
                data.artists.push(range);
            }
            "ALBUM" => data.album = data.text.push_str_get_range(val),
            "ALBUMARTIST" => data.album_artist = data.text.push_str_get_range(val),
            "RATING" => data.rating = TrackRating::from_vorbis_comments(val),
            "TRACKNUMBER" => data.number = val.parse::<u8>().unwrap_or_default(),
            "DATE" => data.date = data.text.push_str_get_range(val),
            _ => {}
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
    pub path: Range<usize>,
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
        self.rating = TrackRating::None;

        let range = Range { start: 0, end: 0 };

        self.path = range;
        self.title = range;
        self.album = range;
        self.album_artist = range;
        self.date = range;
    }
}

#[derive(Default, Clone)]
pub struct AudioProperties {
    pub duration: TrackDuration,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AudioExt {
    #[default]
    Flac,
    Opus,
    Ogg,
    Mp3,
}

impl AudioExt {
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
