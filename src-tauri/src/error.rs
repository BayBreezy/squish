use serde::{Serialize, Serializer};

/// Everything that can go wrong in the image pipeline.
///
/// Tauri sends command errors to the frontend as JSON, so this implements
/// `Serialize` by emitting the human-readable message as a plain string.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Could not read image: {0}")]
    Decode(String),
    #[error("Could not encode image: {0}")]
    Encode(String),
    #[error("{0}")]
    Unsupported(String),
}

impl From<image::ImageError> for AppError {
    fn from(e: image::ImageError) -> Self {
        AppError::Decode(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
