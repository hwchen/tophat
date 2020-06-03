//! Errors that indicate system failure, user error in using tophat, or closed connection.
//!
//! "App" errors, which are handled within an endpoint and result only in loggin and an Http
//! Response, are handled by `Glitch`.

use std::fmt;

/// Public Errors (does not include internal fails)
#[derive(Debug)]
pub enum Error {
    /// Error when converting from a type to Body
    BodyConversion(std::io::Error),

    /// Error because tophat does not support the transfer encoding.
    ConnectionClosedUnsupportedTransferEncoding,

    /// Connection lost
    ConnectionLost(std::io::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use Error::*;
        match self {
            BodyConversion(err) => Some(err),
            ConnectionClosedUnsupportedTransferEncoding => None,
            ConnectionLost(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            BodyConversion(err) => write!(f, "Error converting body: {}", err),
            ConnectionClosedUnsupportedTransferEncoding => {
                write!(f, "Connection closed: Unsupported Transfer Encoding")
            }
            ConnectionLost(err) => write!(f, "Connection lost: {}", err),
        }
    }
}
