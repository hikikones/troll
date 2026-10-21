use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::database2::{AudioExt, TrackId};

type DecodeResult = Result<DecodeSuccess, DecodeFailure>;
type DecodeHandle = std::thread::JoinHandle<DecodeResult>;
type Decoder = rodio::Decoder<BufReader<File>>;

pub struct AudioDecoder {
    handle: Option<DecodeHandle>,
}

impl AudioDecoder {
    pub const fn new() -> Self {
        Self { handle: None }
    }

    pub fn start(&mut self, request: DecodeRequest) {
        self.handle = Some(start_decode(request));
    }

    pub fn poll(&mut self) -> Option<DecodeResult> {
        let Some(handle) = self.handle.as_ref() else {
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

    pub fn clear(&mut self) {
        self.handle = None;
    }
}

pub struct DecodeRequest {
    pub id: TrackId,
    pub index: usize,
    pub path: PathBuf,
}

pub struct DecodeSuccess {
    pub id: TrackId,
    pub index: usize,
    pub decode: Decoder,
}

pub struct DecodeFailure {
    pub id: TrackId,
    pub path: PathBuf,
    pub error: DecodeError,
}

#[derive(Debug)]
pub enum DecodeError {
    Io(std::io::Error),
    Decode(rodio::decoder::DecoderError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Io(err) => err.fmt(f),
            DecodeError::Decode(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for DecodeError {}

fn start_decode(request: DecodeRequest) -> DecodeHandle {
    let DecodeRequest { id, index, path } = request;

    std::thread::spawn(move || match decode_audio(&path) {
        Ok(decode) => Ok(DecodeSuccess { id, index, decode }),
        Err(error) => Err(DecodeFailure { id, path, error }),
    })
}

fn decode_audio(path: &Path) -> Result<Decoder, DecodeError> {
    let ext = AudioExt::from_path(path).unwrap();

    let file = File::open(path).map_err(|err| DecodeError::Io(err))?;
    let len = file.metadata().map_err(|err| DecodeError::Io(err))?.len();
    let decoder = rodio::Decoder::builder()
        .with_data(BufReader::new(file))
        .with_hint(ext.as_lower_case())
        .with_byte_len(len)
        .with_seekable(true)
        .with_gapless(false)
        .build()
        .map_err(|err| DecodeError::Decode(err))?;

    Ok(decoder)
}
