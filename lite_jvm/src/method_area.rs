use crate::bootstrap_class_loader::{BootstrapClassLoader, ClassLoader, LoadClassResult};
use crate::class_finder::ClassPath;
use crate::jvm_exceptions::Result;
use crate::loaded_class::{Class, ClassRef, ClassStatus};
use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::class_file::ClassFile;
use std::cell::RefCell;
use std::collections::HashMap;
use typed_arena::Arena;

/// 方法区的功能抽象，用来管理类的加载->链接->初始化。
/// 需要一个classloader以外的管理者进行对类统一管理。
/// 先暂时不考虑多线程。因为每个线程需要一份类加载器

pub struct MethodArea<'a> {
    bootstrap_class_loader: RefCell<BootstrapClassLoader<'a>>,
    custom_class_loader: HashMap<&'a str, ClassRef<'a>>,
    classes: Arena<Class<'a>>,
}
impl<'a> MethodArea<'a> {
    pub fn new() -> MethodArea<'a> {
        MethodArea {
            bootstrap_class_loader: RefCell::new(BootstrapClassLoader::new()),
            custom_class_loader: HashMap::new(),
            classes: Arena::new(),
        }
    }
    pub fn load_class(&self, class_name: &str) -> Result<ClassRef<'a>> {
        let load_class_result = self
            .bootstrap_class_loader
            .borrow()
            .load_class(class_name)?;
        match load_class_result {
            LoadClassResult::NewLoaded(class) => {
                let class = self.do_class_loading(class)?;
                self.bootstrap_class_loader
                    .borrow_mut()
                    .registry_class(class);
                Ok(class)
            }
            LoadClassResult::AlreadyLoaded(class_ref) => Ok(class_ref),
        }
    }

    fn do_class_loading(&self, class_file: ClassFile) -> Result<ClassRef<'a>> {
        //解析super_class
        let super_class = if let Some(super_class_name) = &class_file.super_class_name {
            let result = self.load_class(&super_class_name)?;
            Some(result)
        } else {
            None
        };
        let mut interfaces = Vec::new();
        //解析加载接口
        for interface_name in &class_file.interface_names {
            let result = self.load_class(interface_name)?;
            interfaces.push(result);
        }
        let class_ref = self.classes.alloc(Class {
            status: ClassStatus::Loaded,
            name: class_file.this_class_name,
            constant_pool: RuntimeConstantPool::from(&class_file.constant_pool)?,
            access_flags: class_file.access_flags,
            super_class,
            interfaces,
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
            super_class_name: class_file.super_class_name,
            interface_names: class_file.interface_names,
        });
        //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = class_ref;
            &*class_ptr
        };
        Ok(class_ref)
    }

    ///
    /// ClassLoader类都会执行此registry_natives 方法。将class_loader类注册进来，
    /// 然后使用该加载器进行加载时需要调用loadClass方法执行。先搁置
    ///
    pub fn registry_natives(&mut self, class_loader: ClassRef<'a>) {
        self.custom_class_loader
            .insert(&class_loader.name, class_loader);
    }

    /// 从命令行读取class_path。然后添加
    pub fn add_class_path(&self, class_path: Box<dyn ClassPath>) {
        self.bootstrap_class_loader
            .borrow_mut()
            .add_class_path(class_path);
    }
}

mod tests {

    #[test]
    fn test_class_load() {
        use crate::class_finder::{FileSystemClassPath, JarFileClassPath};
        use crate::loaded_class::ClassStatus;
        use crate::method_area::MethodArea;
        let area = MethodArea::new();

        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        area.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new(
            "/Library/Java/JavaVirtualMachines/jdk1.8.0_202.jdk/Contents/Home/jre/lib/rt.jar",
        )
        .unwrap();

        area.add_class_path(Box::new(rt_jar_path));
        let result = area.load_class("HelloWorld").unwrap();

        assert!(matches!(result.status, ClassStatus::Loaded));
    }
}
