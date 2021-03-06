//! Error types

#![allow(unused_macros)]

use core::fmt;
#[cfg(feature = "std")]
use std::{
    error::Error as StdError,
    io,
    string::{FromUtf8Error, String, ToString},
};
#[cfg(feature = "encoding")]
use subtle_encoding;

/// Error type
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,

    #[cfg(feature = "std")]
    description: Option<String>,
}

impl Error {
    /// Create a new error object with an optional error message
    #[allow(unused_variables)]
    pub fn new(kind: ErrorKind, description: Option<&str>) -> Self {
        Error {
            kind,

            #[cfg(feature = "std")]
            description: description.map(|desc| desc.to_string()),
        }
    }

    /// Obtain the ErrorKind for this Error
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

#[cfg(not(feature = "std"))]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind.as_str())
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.description {
            Some(ref desc) => write!(f, "{}: {}", self.description(), desc),
            None => write!(f, "{}", self.description()),
        }
    }
}

#[cfg(feature = "std")]
impl StdError for Error {
    fn description(&self) -> &str {
        if let Some(ref desc) = self.description {
            desc
        } else {
            self.kind.as_str()
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,

            #[cfg(feature = "std")]
            description: None,
        }
    }
}

/// Kinds of errors
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ErrorKind {
    /// Input/output error
    Io,

    /// Malformatted or otherwise invalid cryptographic key
    KeyInvalid,

    /// Error parsing a file format or other data
    ParseError,

    /// Internal error within a cryptographic provider
    ProviderError,

    /// Signature is not valid
    SignatureInvalid,
}

impl ErrorKind {
    /// Obtain a string description of an error. Like `description()` but not
    /// bound to `std`
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Io => "I/O error",
            ErrorKind::KeyInvalid => "invalid cryptographic key",
            ErrorKind::ParseError => "parse error",
            ErrorKind::ProviderError => "internal crypto provider error",
            ErrorKind::SignatureInvalid => "bad signature",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Create a new error (of a given enum variant) with a formatted message
#[cfg(not(feature = "std"))]
macro_rules! err {
    ($variant:ident, $msg:expr) => {
        ::error::Error::from(::error::ErrorKind::$variant)
    };
    ($variant:ident, $fmt:expr, $($arg:tt)+) => {
        ::error::Error::from(::error::ErrorKind::$variant)
    };
}

/// Create a new error (of a given enum variant) with a formatted message
#[cfg(feature = "std")]
macro_rules! err {
    ($variant:ident, $msg:expr) => {
        ::error::Error::new(
            ::error::ErrorKind::$variant,
            Some($msg)
        )
    };
    ($variant:ident, $fmt:expr, $($arg:tt)+) => {
        err!($variant, &format!($fmt, $($arg)+))
    };
}

/// Create and return an error with a formatted message
#[allow(unused_macros)]
macro_rules! fail {
    ($kind:ident, $msg:expr) => {
        return Err(err!($kind, $msg).into());
    };
    ($kind:ident, $fmt:expr, $($arg:tt)+) => {
        return Err(err!($kind, $fmt, $($arg)+).into());
    };
}

/// Assert a condition is true, returning an error type with a formatted message if not
macro_rules! ensure {
    ($condition: expr, $variant:ident, $msg:expr) => {
        if !($condition) {
            return Err(err!($variant, $msg).into());
        }
    };
    ($condition: expr, $variant:ident, $fmt:expr, $($arg:tt)+) => {
        if !($condition) {
            return Err(err!($variant, $fmt, $($arg)+).into());
        }
    };
}

#[cfg(feature = "std")]
impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        err!(ParseError, &err.to_string())
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        err!(Io, &err.to_string())
    }
}

#[cfg(feature = "encoding")]
impl From<subtle_encoding::Error> for Error {
    fn from(err: subtle_encoding::Error) -> Self {
        match err {
            subtle_encoding::Error::EncodingInvalid => err!(ParseError, "invalid encoding"),
            subtle_encoding::Error::LengthInvalid => err!(ParseError, "invalid length"),
            #[cfg(feature = "std")]
            subtle_encoding::Error::IoError => err!(Io, &err.to_string()),
        }
    }
}
