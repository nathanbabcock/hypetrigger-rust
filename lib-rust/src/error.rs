use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;
use std::sync::mpsc::SendError;
use std::sync::PoisonError;

use tesseract::plumbing::{TessBaseApiGetUtf8TextError, TessBaseApiSetImageSafetyError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub source: Option<Box<dyn StdError + 'static>>,
    // Would an `Arc` be nice here, to cross thread boundaries?
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref()
    }
}

impl Error {
    /// Wrap any standard Error into a library Error.
    /// Similar to [`anyhow`](https://github.com/dtolnay/anyhow/blob/master/src/error.rs#L88).
    pub fn from_std<E>(e: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Error {
            message: e.to_string(),
            source: Some(Box::new(e)),
        }
    }

    /// Wrap any Display into a library Error.
    pub fn from_display<E>(e: E) -> Self
    where
        E: Display,
    {
        Error {
            message: e.to_string(),
            source: None,
        }
    }
}

/// Represents an attempt to unwrap a None value from an Option.
///
/// ```rs
/// let value = Some(x).ok_or(NoneError)?;
/// ```
#[derive(Debug)]
pub struct NoneError;
impl Display for NoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "called unwrap() on None")
    }
}
impl std::error::Error for NoneError {}

impl From<NoneError> for Error {
    fn from(e: NoneError) -> Self {
        Error::from_std(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::from_std(e)
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::from_std(e)
    }
}

impl<T: Send + 'static> From<SendError<T>> for Error {
    fn from(e: SendError<T>) -> Self {
        Error::from_std(e)
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(e: PoisonError<T>) -> Self {
        // Because `PoisonError` keeps a (non-static) reference to `self`, which
        // can't be allowed to cross function boundaries, skip the `source` field.
        Error::from_display(e.to_string())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::from_display(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::from_display(e)
    }
}

#[cfg(feature = "tesseract")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::from_std(e)
    }
}

#[cfg(feature = "tesseract")]
impl From<tesseract::InitializeError> for Error {
    fn from(e: tesseract::InitializeError) -> Self {
        Error::from_std(e)
    }
}

#[cfg(feature = "tesseract")]
impl From<TessBaseApiSetImageSafetyError> for Error {
    fn from(e: TessBaseApiSetImageSafetyError) -> Self {
        Error::from_std(e)
    }
}

#[cfg(feature = "tesseract")]
impl From<TessBaseApiGetUtf8TextError> for Error {
    fn from(e: TessBaseApiGetUtf8TextError) -> Self {
        Error::from_std(e)
    }
}

#[cfg(feature = "tensorflow")]
impl From<tensorflow::Status> for Error {
    fn from(e: tensorflow::Status) -> Self {
        Error::from_std(e)
    }
}
