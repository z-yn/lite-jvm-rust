use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum Exception {
    ClassNotFoundException(String),
    NoClassDefFoundError(String),

    ClassPathNotExist(String),
    JarFileNotExist(String),
    ReadClassBytesError(Box<dyn Error>),
    ReadJarFileError(Box<dyn Error>),
}

impl Display for Exception {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Exception::ClassNotFoundException(class_name) => {
                write!(f, "ClassNotFoundException:{}", class_name)
            }

            Exception::NoClassDefFoundError(class_name) => {
                write!(f, "NoClassDefFoundError:{}", class_name)
            }
            Exception::ClassPathNotExist(path) => {
                write!(f, "ClassPathNotExist:{}", path)
            }
            Exception::JarFileNotExist(path) => {
                write!(f, "JarFileNotExist:{}", path)
            }
            Exception::ReadClassBytesError(e) => {
                write!(f, "ReadClassBytesError: \n caused by {}", e)
            }
            Exception::ReadJarFileError(e) => {
                write!(f, "ReadJarFileError: \n caused by {}", e)
            }
        }
    }
}

impl Error for Exception {}

pub type Result<T> = std::result::Result<T, Exception>;
