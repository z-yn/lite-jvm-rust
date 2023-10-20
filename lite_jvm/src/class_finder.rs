/// 设计上需要定义一个类查找其
use crate::jvm_exceptions::{Exception, Result};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use zip::result::ZipError;
use zip::ZipArchive;

pub struct ClassFinder {
    pub class_paths: Vec<Box<dyn ClassPath>>,
}

impl ClassFinder {
    pub fn new() -> ClassFinder {
        ClassFinder {
            class_paths: Vec::new(),
        }
    }
    //查找class,如果查找失败则返回ClassNotFoundException
    pub fn find_class(&self, name: &str) -> Result<Vec<u8>> {
        for class_path in &self.class_paths {
            if let Some(v) = class_path.find_class(name)? {
                return Ok(v);
            }
        }
        Err(Exception::ClassNotFoundException(String::from(name)))
    }
}

/// 定义一个能够查找类路径的结构
pub trait ClassPath {
    //根据名字查找class,可能查的到。也可能找不到。
    fn find_class(&self, class_name: &str) -> Result<Option<Vec<u8>>>;
}

//通过本地路径进行加载，支持绝对路径和相对路径。
pub struct FileSystemClassPath {
    class_path_root: PathBuf,
}

impl FileSystemClassPath {
    pub fn new(path: &str) -> Result<FileSystemClassPath> {
        let class_path_root = if let Ok(abs_path) = fs::canonicalize(PathBuf::from(path)) {
            PathBuf::from(abs_path)
        } else {
            return Err(Exception::ClassPathNotExist(path.to_string()));
        };

        if !class_path_root.exists() || !class_path_root.is_dir() {
            Err(Exception::ClassPathNotExist(
                class_path_root.to_string_lossy().to_string(),
            ))
        } else {
            Ok(Self { class_path_root })
        }
    }
}

impl ClassPath for FileSystemClassPath {
    fn find_class(&self, class_name: &str) -> Result<Option<Vec<u8>>> {
        let mut full_path = self.class_path_root.clone();
        full_path.push(class_name);
        full_path.set_extension("class");
        if full_path.exists() {
            fs::read(full_path)
                .map(Some)
                .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))
        } else {
            Ok(None)
        }
    }
}

//支持从jar包内加载，jar包本质上是个zip文件

pub struct JarFileClassPath {
    jar_file_path: String,
    zip: RefCell<ZipArchive<BufReader<File>>>,
}

impl Debug for JarFileClassPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "JarFileClassPath => {}", &self.jar_file_path)
    }
}

impl JarFileClassPath {
    pub fn new(path: &str) -> Result<JarFileClassPath> {
        let jar_file_path = if let Ok(abs_path) = fs::canonicalize(PathBuf::from(path)) {
            PathBuf::from(abs_path)
        } else {
            return Err(Exception::JarFileNotExist(path.to_string()));
        };

        if !jar_file_path.exists() {
            Err(Exception::JarFileNotExist(
                jar_file_path.to_string_lossy().to_string(),
            ))
        } else {
            let file =
                File::open(&jar_file_path).map_err(|e| Exception::ReadJarFileError(Box::new(e)))?;
            let buf_reader = BufReader::new(file);
            let zip = ZipArchive::new(buf_reader)
                .map_err(|e| Exception::ReadJarFileError(Box::new(e)))?;
            Ok(Self {
                jar_file_path: jar_file_path.to_string_lossy().to_string(),
                zip: RefCell::new(zip),
            })
        }
    }
}

impl ClassPath for JarFileClassPath {
    fn find_class(&self, class_name: &str) -> Result<Option<Vec<u8>>> {
        let class_file_name = class_name.to_string() + ".class";
        match self.zip.borrow_mut().by_name(&class_file_name) {
            Ok(mut zip_file) => {
                let mut buffer: Vec<u8> = Vec::with_capacity(zip_file.size() as usize);
                zip_file
                    .read_to_end(&mut buffer)
                    .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))?;
                Ok(Some(buffer))
            }
            Err(ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(Exception::ReadClassBytesError(Box::new(e))),
        }
    }
}
#[allow(unused_imports)]
mod tests {
    use crate::class_finder::{ClassPath, FileSystemClassPath, JarFileClassPath};
    use class_file_reader::class_file_reader::read_buffer;

    #[test]
    fn test_file_system_class_finding() {
        let result = FileSystemClassPath::new("./resources").unwrap();
        let x = result.find_class("HelloWorld").unwrap();
        assert!(x.is_some());
        let parsed_files = read_buffer(&x.unwrap()).unwrap();
        assert_eq!(parsed_files.this_class_name, "HelloWorld");
        let not_exist = result.find_class("java/lang/String").unwrap();
        assert!(not_exist.is_none());
    }

    #[test]
    fn test_jar_file_class_finding() {
        let result = JarFileClassPath::new("./resources/rt.jar").unwrap();
        let string_file = result.find_class("java/lang/Object").unwrap();
        assert!(string_file.is_some());
        let parsed_files = read_buffer(&string_file.unwrap()).unwrap();
        assert_eq!(parsed_files.this_class_name, "java/lang/Object");
        let not_exist = result.find_class("Hello").unwrap();
        assert!(not_exist.is_none());
    }
}
