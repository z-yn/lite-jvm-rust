use class_file_reader::class_file_error::ClassFileError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum VmError {
    #[error("ClassFormatError")]
    ClassFormatError,
    #[error("ClassNotFoundException {0}")]
    ClassNotFoundException(String),
    #[error("MethodNotFoundException {0} {1}")]
    MethodNotFoundException(String, String),
    #[error("FieldNotFoundException {0}")]
    FieldNotFoundException(String),
    #[error("InvalidAttribute {0}")]
    InvalidAttribute(String),
    #[error("InvalidAttribute {0}")]
    InvalidOffset(usize),
    #[error("NoClassDefFoundError {0}")]
    NoClassDefFoundError(String),
    #[error("ClassPathNotExist {0}")]
    ClassPathNotExist(String),
    #[error("JarFileNotExist {0}")]
    JarFileNotExist(String),
    #[error("JarFileNotExist {0}")]
    ReadClassBytesError(String),
    #[error("ExecuteCodeError {0}")]
    ExecuteCodeError(String),
    #[error("value type miss match")]
    ValueTypeMissMatch,
    #[error("ReadJarFileError {0}")]
    ReadJarFileError(String),
    #[error("VersionNotSupport")]
    ClassVersionNotSupport,
    #[error("index out of bounds")]
    IndexOutOfBounds,
    #[error("can't pop from empty stack")]
    PopFromEmptyStack,
    #[error("stack over flow")]
    StackOverFlow,
    #[error("arithmetic error")]
    ArithmeticException,
    #[error("NotImplemented error")]
    NotImplemented,
}

pub type VmExecResult<T> = Result<T, VmError>;

impl<'a> From<ClassFileError> for VmError {
    fn from(value: ClassFileError) -> Self {
        VmError::ReadClassBytesError(value.to_string())
    }
}
