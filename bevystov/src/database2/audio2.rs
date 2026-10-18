use std::{fs::File, path::Path, time::Duration};

use lofty::{
    config::ParseOptions,
    file::AudioFile as LoftyAudioFile,
    flac::FlacFile,
    ogg::{OpusFile, VorbisFile, tag::VorbisComments},
};

pub type AudioResult<T> = Result<T, AudioFileError>;

pub struct AudioFile(AudioFormat);

enum AudioFormat {
    Flac(FlacFile),
    Opus(OpusFile),
    Vorbis(VorbisFile),
}

impl AudioFile {
    pub fn read(path: impl AsRef<Path>, ext: AudioExt) -> AudioResult<Self> {
        let mut file = File::open(path.as_ref())?;

        let options = ParseOptions::new().read_cover_art(false);
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

    pub fn properties(&self) -> AudioProperties {
        match &self.0 {
            AudioFormat::Flac(flac) => {
                let p = flac.properties();
                AudioProperties {
                    duration: p.duration(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: Some(p.bit_depth()),
                    sample_rate_khz: Some(p.sample_rate()),
                }
            }
            AudioFormat::Opus(opus) => {
                let p = opus.properties();
                AudioProperties {
                    duration: p.duration(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate_khz: None,
                }
            }
            AudioFormat::Vorbis(vorbis) => {
                let p = vorbis.properties();
                AudioProperties {
                    duration: p.duration(),
                    bit_rate_kbps: p.audio_bitrate(),
                    bit_depth: None,
                    sample_rate_khz: Some(p.sample_rate()),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AudioProperties {
    pub duration: Duration,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate_khz: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub enum AudioExt {
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
                    Some(Self::Opus)
                }
                None
            }
        })
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
    MissingVorbisComments,
}

impl std::fmt::Display for AudioFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFileError::Io(err) => err.fmt(f),
            AudioFileError::Parse(err) => err.fmt(f),
            AudioFileError::MissingVorbisComments => {
                f.write_str("No VorbisComments metadata container found")
            }
        }
    }
}

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

impl std::error::Error for AudioFileError {}
