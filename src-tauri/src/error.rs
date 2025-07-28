use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to parse as string: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid path")]
    InvalidPath,

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
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Io(String),
    Utf8(String),
    InvalidPath,
    Symphonia(String),
    HoundWriteError(String),
    NoDefaultTrackFound(String),
    NoAudioData(String),
    PlaybackError(String),
    TauriError(String),
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
            Self::Symphonia(_) => ErrorKind::Symphonia(error_message),
            Self::HoundWriteError(_) => ErrorKind::HoundWriteError(error_message),
            Self::NoDefaultTrackFound => ErrorKind::NoDefaultTrackFound(error_message),
            Self::NoAudioData => ErrorKind::NoAudioData(error_message),
            Self::PlaybackError => ErrorKind::PlaybackError(error_message),
            Self::TauriError(_) => ErrorKind::TauriError(error_message),
        };
        error_kind.serialize(serializer)
    }
}
