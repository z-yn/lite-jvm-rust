use crate::bootstrap_class_loader::{BootstrapClassLoader, ClassLoader, LoadClassResult};
use crate::class_finder::ClassPath;
use crate::jvm_exceptions::Result;
use crate::loaded_class::{ClassRef, ClassStatus, LoadedClass};
use std::collections::HashMap;

/// 用来管理类的加载->链接->初始化
/// 需要一个classloader以外的管理者进行对类统一管理。
/// 先暂时不考虑多线程。因为每个线程需要一份类加载器

pub struct ClassManager<'a> {
    bootstrap_class_loader: BootstrapClassLoader<'a>,
    custom_class_loader: HashMap<&'a String, ClassRef<'a>>,
}
impl<'a> ClassManager<'a> {
    pub(crate) fn new() -> ClassManager<'a> {
        ClassManager {
            bootstrap_class_loader: BootstrapClassLoader::new(),
            custom_class_loader: HashMap::new(),
        }
    }
    fn load_class<'b: 'a>(&'b mut self, class_name: &str) -> Result<ClassRef<'a>> {
        match self.bootstrap_class_loader.load_class(class_name)? {
            LoadClassResult::NewLoaded(mut class) => {
                self.link_class(&mut class);
                self.initialize_class(&mut class);
                Ok(self
                    .bootstrap_class_loader
                    .registry_class(class_name, class)?)
            }
            LoadClassResult::AlreadyLoaded(class_ref) => Ok(class_ref),
        }
    }

    fn link_class(&self, class: &mut LoadedClass<'a>) {
        if let ClassStatus::Loaded = class.status {
            //do link
            class.status = ClassStatus::Linked;
        }
    }

    fn initialize_class(&self, class: &mut LoadedClass<'a>) {
        if let ClassStatus::Linked = class.status {
            //do init
            class.status = ClassStatus::Initialized;
        }
    }

    ///
    /// ClassLoader类都会执行此registry_natives 方法。将class_loader类注册进来，
    /// 然后使用该加载器进行加载时需要调用loadClass方法执行。先搁置
    ///
    fn registry_natives(&mut self, class_loader: ClassRef<'a>) {
        self.custom_class_loader
            .insert(&class_loader.name, class_loader);
    }

    /// 从命令行读取class_path。然后添加
    fn add_class_path(&mut self, class_path: Box<dyn ClassPath>) {
        self.bootstrap_class_loader.add_class_path(class_path);
    }
}
