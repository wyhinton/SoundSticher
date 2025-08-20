use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to parse as string: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid path")]
    InvalidPath,

    #[error("Unknown encoder format: {0}")]
    UnknownEncoderFormat(String),

    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),

    #[error(transparent)]
    HoundWriteError(#[from] hound::Error),

    #[error(transparent)]
    TauriError(#[from] tauri::Error),

    #[error("No default track found for")]
    NoDefaultTrackFound,

    #[error("No audio data")]
    NoAudioData,

    #[error("No audio data")]
    PlaybackError,

    #[error("MP3 encoder build error: {0}")]
    MP3EncoderError(String),

    #[error("Uneven Number of Samples Provided")]
    UnevenNumberOfSamples,

    #[error("FLAC encode error: {0}")]
    FlacEncodeError(String),

    #[error("FLAC output error: {0}")]
    FlacOutputError(String),
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Io(String),
    Utf8(String),
    InvalidPath,
    UnknownEncoderFormat(String),
    Symphonia(String),
    HoundWriteError(String),
    NoDefaultTrackFound(String),
    NoAudioData(String),
    PlaybackError(String),
    TauriError(String),
    MP3EncoderError(String),
    UnevenNumberOfSamples,
    FlacEncodeError(String),
    FlacOutputError(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let error_message = self.to_string();
        let error_kind = match self {
            Self::Io(_) => ErrorKind::Io(error_message),
            Self::Utf8(_) => ErrorKind::Utf8(error_message),
            Self::InvalidPath => ErrorKind::InvalidPath,
            Self::UnknownEncoderFormat(_) => ErrorKind::UnknownEncoderFormat(error_message),
            Self::Symphonia(_) => ErrorKind::Symphonia(error_message),
            Self::HoundWriteError(_) => ErrorKind::HoundWriteError(error_message),
            Self::NoDefaultTrackFound => ErrorKind::NoDefaultTrackFound(error_message),
            Self::NoAudioData => ErrorKind::NoAudioData(error_message),
            Self::PlaybackError => ErrorKind::PlaybackError(error_message),
            Self::TauriError(_) => ErrorKind::TauriError(error_message),
            Self::MP3EncoderError(_) => ErrorKind::MP3EncoderError(error_message),
            Self::UnevenNumberOfSamples => ErrorKind::UnevenNumberOfSamples,
            Self::FlacEncodeError(_) => ErrorKind::FlacEncodeError(error_message),
            Self::FlacOutputError(_) => ErrorKind::FlacOutputError(error_message),
        };
        error_kind.serialize(serializer)
    }
}
