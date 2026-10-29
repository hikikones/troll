use std::{fs::File, path::Path, time::Duration};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile as LoftyAudioFile,
    flac::FlacFile,
    ogg::{OggPictureStorage, OpusFile, VorbisFile, tag::VorbisComments},
    picture::PictureType,
};

pub type AudioResult<T> = Result<T, AudioFileError>;

pub struct AudioFile(AudioFormat);

enum AudioFormat {
    Flac(FlacFile),
    Opus(OpusFile),
    Vorbis(VorbisFile),
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
        };

        Ok(Self(audio_format))
    }

    pub fn vorbis_comments(&self) -> AudioResult<&VorbisComments> {
        match &self.0 {
            AudioFormat::Flac(flac) => flac
                .vorbis_comments()
                .ok_or(AudioFileError::MissingVorbisComments),
            AudioFormat::Opus(opus) => Ok(opus.vorbis_comments()),
            AudioFormat::Vorbis(vorbis) => Ok(vorbis.vorbis_comments()),
        }
    }

    pub fn vorbis_comments_mut(&mut self) -> AudioResult<&mut VorbisComments> {
        match &mut self.0 {
            AudioFormat::Flac(flac) => flac
                .vorbis_comments_mut()
                .ok_or(AudioFileError::MissingVorbisComments),
            AudioFormat::Opus(opus) => Ok(opus.vorbis_comments_mut()),
            AudioFormat::Vorbis(vorbis) => Ok(vorbis.vorbis_comments_mut()),
        }
    }

    pub fn properties(&self) -> AudioProperties {
        match &self.0 {
            AudioFormat::Flac(flac) => {
                let p = flac.properties();
                AudioProperties {
                    duration: p.duration(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: Some(p.bit_depth()),
                    sample_rate: Some(p.sample_rate()),
                }
            }
            AudioFormat::Opus(opus) => {
                let p = opus.properties();
                AudioProperties {
                    duration: p.duration(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate: None,
                }
            }
            AudioFormat::Vorbis(vorbis) => {
                let p = vorbis.properties();
                AudioProperties {
                    duration: p.duration(),
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
        };

        pictures
            .iter()
            .find(|(picture, _)| picture.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures.first())
            .map(|(picture, _)| picture.data())
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
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct AudioProperties {
    pub duration: Duration,
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
}

impl AudioExt {
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| {
            if ext.eq_ignore_ascii_case("flac") {
                Some(Self::Flac)
            } else if ext.eq_ignore_ascii_case("ogg") {
                Some(Self::Ogg)
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
        }
    }

    pub const fn as_upper_case(self) -> &'static str {
        match self {
            Self::Flac => "FLAC",
            Self::Opus => "OPUS",
            Self::Ogg => "OGG",
        }
    }
}

#[derive(Debug)]
pub enum AudioFileError {
    Io(std::io::Error),
    Parse(lofty::error::FileParseError),
    Encode(lofty::error::FileEncodingError),
    MissingVorbisComments,
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
