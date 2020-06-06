//! Errors that indicate system failure, user error in using tophat, or closed connection.
//!
//! "App" errors, which are handled within an endpoint and result only in loggin and an Http
//! Response, are handled by `Glitch`.

use std::fmt;

/// Public Errors (does not include internal fails)
#[derive(Debug)]
pub enum ServerError {
    /// Error because tophat does not support the transfer encoding.
    ConnectionClosedUnsupportedTransferEncoding,

    /// Connection lost
    ConnectionLost(std::io::Error),
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use ServerError::*;
        match self {
            ConnectionClosedUnsupportedTransferEncoding => None,
            ConnectionLost(err) => Some(err),
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ServerError::*;
        match self {
            ConnectionClosedUnsupportedTransferEncoding => {
                write!(f, "Connection closed: Unsupported Transfer Encoding")
            }
            ConnectionLost(err) => write!(f, "Connection lost: {}", err),
        }
    }
}
