use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// Models the possible errors returned when reading a .class file
#[derive(Debug, PartialEq, Eq)]
pub enum ClassFileError {
    InvalidClassData(String),
    UnsupportedVersion(u16, u16),

    ConstantPoolTagNotSupport(u8),
    InvalidConstantPoolIndexError(u16),
    InvalidMethodHandlerKind(u8),

    UnexpectedEndOfData,
    InvalidCesu8String,

    InvalidCode(String),
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
            ClassFileError::InvalidClassData(msg) => write!(f, "invalid class data: {msg}"),
            ClassFileError::UnexpectedEndOfData => write!(f, "unexpected end of data"),
            ClassFileError::InvalidCesu8String => write!(f, "invalid cesu8 string"),
            ClassFileError::ConstantPoolTagNotSupport(tag) => {
                write!(f, "constant pool tag not support: {tag}")
            }
            ClassFileError::InvalidCode(msg) => {
                write!(f, "invalid code : {msg}")
            }
        }
    }
}

impl Error for ClassFileError {}

pub type Result<T> = std::result::Result<T, ClassFileError>;
