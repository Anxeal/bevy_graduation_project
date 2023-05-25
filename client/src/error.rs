use std::error::Error as StdError;
use std::fmt;
pub type Result<T> = ::std::result::Result<T, Error>;
pub type Error = Box<ErrorKind>;

#[derive(Debug)]
pub enum ErrorKind {
    Serialization(bincode::Error),
    Network(tungstenite::Error),
}

impl StdError for ErrorKind {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            ErrorKind::Serialization(ref err) => Some(err),
            ErrorKind::Network(ref err) => Some(err),
        }
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

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::Serialization(ref err) => write!(fmt, "serialization error: {}", err),
            ErrorKind::Network(ref err) => write!(fmt, "network error: {}", err),
        }
    }
}
