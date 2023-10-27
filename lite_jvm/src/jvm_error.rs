use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VmError {
    #[error("ClassNotFoundException {0}")]
    ClassNotFoundException(String),
    #[error("MethodNotFoundException {0}")]
    MethodNotFoundException(String),
    #[error("FieldNotFoundException {0}")]
    FieldNotFoundException(String),
    #[error("InvalidAttribute {0}")]
    InvalidAttribute(String),
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

    #[error("index out of bounds")]
    IndexOutOfBounds,
    #[error("can't pop from empty stack")]
    PopFromEmptyStack,
    #[error("stack over flow")]
    StackOverFlow,
}

pub type VmExecResult<T> = Result<T, VmError>;
