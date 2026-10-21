use std::{io::Cursor, path::PathBuf};

use rodio::Decoder;

use crate::database2::{AudioExt, TrackId};

type DecodeResult = Result<DecodeSuccess, DecodeError>;
type DecodeHandle = std::thread::JoinHandle<DecodeResult>;

pub(super) struct AudioDecoder {
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

pub(super) struct DecodeRequest {
    pub id: TrackId,
    pub index: usize,
    pub path: PathBuf,
}

pub(super) struct DecodeSuccess {
    pub id: TrackId,
    pub index: usize,
    pub data: Decoder<Cursor<Vec<u8>>>,
}

pub(super) struct DecodeError {
    pub id: TrackId,
    pub path: PathBuf,
    pub source: DecodeErrorSource,
}

#[derive(Debug)]
pub(super) enum DecodeErrorSource {
    Io(std::io::Error),
    Decode(rodio::decoder::DecoderError),
}

impl std::fmt::Display for DecodeErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeErrorSource::Io(err) => err.fmt(f),
            DecodeErrorSource::Decode(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for DecodeErrorSource {}

fn start_decode(request: DecodeRequest) -> DecodeHandle {
    let ext = AudioExt::from_path(&request.path).unwrap();
    std::thread::spawn(move || {
        let bytes = match std::fs::read(&request.path) {
            Ok(bytes) => bytes,
            Err(err) => {
                return Err(DecodeError {
                    id: request.id,
                    path: request.path,
                    source: DecodeErrorSource::Io(err),
                });
            }
        };

        let len = bytes.len() as u64;
        let decoder = Decoder::builder()
            .with_data(Cursor::new(bytes))
            .with_hint(ext.as_lower_case())
            .with_byte_len(len)
            .with_seekable(true)
            .with_gapless(false)
            .build();

        match decoder {
            Ok(data) => Ok(DecodeSuccess {
                id: request.id,
                index: request.index,
                data,
            }),
            Err(err) => Err(DecodeError {
                id: request.id,
                path: request.path,
                source: DecodeErrorSource::Decode(err),
            }),
        }
    })
}
