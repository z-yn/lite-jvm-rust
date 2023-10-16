use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// Models the possible errors returned when reading a .class file
#[derive(Debug, PartialEq, Eq)]
pub enum ClassFileError {
    UnsupportedVersion(u16, u16),
    InvalidConstantPoolIndexError(u16),
    InvalidMethodHandlerKind(u8),
}

impl Display for ClassFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassFileError::UnsupportedVersion(major, minor) => {
                write!(f, "unsupported class file version {major}.{minor}")
            }
            ClassFileError::InvalidConstantPoolIndexError(index) => {
                write!(f, "invalid const pool index {index}")
            }
            ClassFileError::InvalidMethodHandlerKind(kind) => {
                write!(f, "invalid method handler kind {kind}")
            }
        }
    }
}

impl Error for ClassFileError {}

pub type Result<T> = std::result::Result<T, ClassFileError>;
