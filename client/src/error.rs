use std::error::Error as StdError;
use std::fmt;
pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<ErrorKind>;

#[derive(Debug)]
pub enum ErrorKind {
    Io(std::io::Error),
    Serialization(bincode::Error),
    Network(tungstenite::Error),
    Compression(flate2::CompressError),
    Decmpression(flate2::DecompressError),
}

impl StdError for ErrorKind {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::Serialization(ref err) => Some(err),
            ErrorKind::Network(ref err) => Some(err),
            ErrorKind::Compression(ref err) => Some(err),
            ErrorKind::Decmpression(ref err) => Some(err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        ErrorKind::Io(err).into()
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        ErrorKind::Serialization(err).into()
    }
}

impl From<tungstenite::Error> for Error {
    fn from(err: tungstenite::Error) -> Error {
        ErrorKind::Network(err).into()
    }
}

impl From<flate2::CompressError> for Error {
    fn from(err: flate2::CompressError) -> Error {
        ErrorKind::Compression(err).into()
    }
}

impl From<flate2::DecompressError> for Error {
    fn from(err: flate2::DecompressError) -> Error {
        ErrorKind::Decmpression(err).into()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::Io(ref err) => write!(fmt, "I/O error: {}", err),
            ErrorKind::Serialization(ref err) => write!(fmt, "serialization error: {}", err),
            ErrorKind::Network(ref err) => write!(fmt, "network error: {}", err),
            ErrorKind::Compression(ref err) => write!(fmt, "compression error: {}", err),
            ErrorKind::Decmpression(ref err) => write!(fmt, "decompression error: {}", err),
        }
    }
}
