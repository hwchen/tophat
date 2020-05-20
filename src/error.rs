//! Errors that indicate system failure, user error in using tophat, or closed connection.
//!
//! "App" errors, which are handled within an endpoint and result only in loggin and an Http
//! Response, are handled by `Glitch`.

use thiserror::Error as ThisError;

/// Public Errors (does not include internal fails)
#[derive(ThisError, Debug)]
pub enum Error {
    /// Error when converting from a type to Body
    #[error("Error converting body: {0}")]
    BodyConversion(std::io::Error),

    /// Error because tophat does not support the transfer encoding.
    #[error("Connection closed: Unsupported Transfer Encoding")]
    ConnectionClosedUnsupportedTransferEncoding,

    /// Connection lost
    #[error("Connection lost: {0}")]
    ConnectionLost(std::io::Error),
}
