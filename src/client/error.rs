use std::error::Error as StdError;
use std::fmt;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct ClientError {
    kind: Kind,
    source: Option<BoxError>,
}

impl ClientError {
    pub(crate) fn new<E: Into<BoxError>>(kind: Kind, err: Option<E>) -> Self {
        Self {
            kind,
            source: err.map(Into::into),
        }
    }

    //// Returns the status code, if the error was generated from a response.
    //pub fn status(&self) -> Option<StatusCode> {
    //    match self.kind {
    //        Kind::Status(code) => Some(code),
    //        _ => None,
    //    }
    //}
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Kind::*;
        match &self.kind {
            // TODO improve these messages
            Encode(msg) => {
                if let Some(ref err) = self.source {
                    write!(f, "{:?}: {}", msg, err)
                } else {
                    write!(f, "{:?}", msg)
                }
            }
            Decode(msg) => {
                if let Some(ref err) = self.source {
                    write!(f, "{:?}: {}", msg, err)
                } else {
                    write!(f, "{:?}", msg)
                }
            }
            Io => {
                if let Some(ref err) = self.source {
                    write!(f, "Io Error: {}", err)
                } else {
                    write!(f, "Io Error")
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Kind {
    Encode(Option<String>),
    Decode(Option<String>),
    Io,
    //Status(StatusCode),
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|e| &**e as _)
    }
}

pub(crate) fn encode<S: Into<Option<String>>>(msg: S) -> ClientError {
    ClientError::new(Kind::Encode(msg.into()), None::<ClientError>)
}

pub(crate) fn encode_io<E: Into<BoxError>>(err: E) -> ClientError {
    ClientError::new(Kind::Encode(None), Some(err))
}

pub(crate) fn decode<S: Into<Option<String>>>(msg: S) -> ClientError {
    ClientError::new(Kind::Decode(msg.into()), None::<ClientError>)
}

pub(crate) fn decode_err<E: Into<BoxError>>(err: E) -> ClientError {
    ClientError::new(Kind::Decode(None), Some(err))
}

pub(crate) fn io<E: Into<BoxError>>(err: E) -> ClientError {
    ClientError::new(Kind::Io, Some(err))
}
