use crate::bootstrap_class_loader::{BootstrapClassLoader, ClassLoader, LoadClassResult};
use crate::class_finder::ClassPath;
use crate::jvm_error::VmExecResult;
use crate::loaded_class::{Class, ClassRef, ClassStatus};
use crate::runtime_constant_pool::RuntimeConstantPool;
use crate::runtime_field_info::RuntimeFieldInfo;
use crate::runtime_method_info::{MethodKey, RuntimeMethodInfo};
use class_file_reader::class_file::ClassFile;
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::HashMap;
use typed_arena::Arena;

/// 方法区的功能抽象，用来管理类的加载->链接->初始化。
/// 需要一个classloader以外的管理者进行对类统一管理。
pub struct MethodArea<'a> {
    bootstrap_class_loader: RefCell<BootstrapClassLoader<'a>>,
    custom_class_loader: HashMap<&'a str, ClassRef<'a>>,
    classes: Arena<Class<'a>>,
}
impl<'a> Default for MethodArea<'a> {
    fn default() -> Self {
        MethodArea {
            bootstrap_class_loader: RefCell::new(BootstrapClassLoader::new()),
            custom_class_loader: HashMap::new(),
            classes: Arena::new(),
        }
    }
}
impl<'a> MethodArea<'a> {
    pub fn num_of_classes(&self) -> usize {
        self.classes.len()
    }

    pub fn get_mut(&mut self, class_ref: ClassRef<'a>) -> Option<&'a mut Class<'a>> {
        for mut_ref in self.classes.iter_mut() {
            let v1 = mut_ref as *const Class;
            let v2 = class_ref as *const Class;
            if v1 == v2 {
                let ptr = unsafe {
                    let str_ptr: *mut Class = mut_ref;
                    &mut *str_ptr
                };
                return Some(ptr);
            }
        }
        None
    }

    pub fn is_class_loaded(&self, class_name: &str) -> bool {
        self.bootstrap_class_loader.borrow().exist(class_name)
    }
    pub fn load_class(&self, class_name: &str) -> VmExecResult<ClassRef<'a>> {
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

    fn do_class_loading(&self, class_file: ClassFile) -> VmExecResult<ClassRef<'a>> {
        let mut super_num_of_fields: usize = 0;
        //解析super_class
        let super_class = if let Some(super_class_name) = &class_file.super_class_name {
            let result = self.load_class(&super_class_name)?;
            super_num_of_fields = result.total_num_of_fields;
            Some(result)
        } else {
            None
        };

        let mut interfaces = IndexMap::new();
        //解析加载接口
        for interface_name in &class_file.interface_names {
            let result = self.load_class(interface_name)?;
            //我会确保map的key与Value中的name保持一致
            let key = unsafe {
                let str_ptr: *const str = result.name.as_str();
                &*str_ptr
            };
            interfaces.insert(key, result);
        }
        let constant_pool = RuntimeConstantPool::from(&class_file.constant_pool)?;
        let mut fields = IndexMap::new();
        let mut field_offset = 0;
        for field_info in class_file.field_info {
            let mut field = RuntimeFieldInfo::from(field_info, &constant_pool)?;
            //我会确保map的key与Value中的name保持一致
            let key = unsafe {
                let str_ptr: *const str = field.name.as_str();
                &*str_ptr
            };
            if !field.is_static() {
                field_offset += 1;
                field.offset = super_num_of_fields + field_offset;
            }
            fields.insert(key, field);
        }
        let mut methods = IndexMap::new();
        for method_info in class_file.method_info {
            let method = RuntimeMethodInfo::from(method_info, &constant_pool)?;
            methods.insert(MethodKey::by_method(&method), method);
        }
        let class_ref = self.classes.alloc(Class {
            version: class_file.version,
            total_num_of_fields: super_num_of_fields + fields.len(),
            status: ClassStatus::Loaded,
            name: class_file.this_class_name,
            constant_pool,
            access_flags: class_file.access_flags,
            super_class,
            interfaces,
            fields,
            methods,
            super_class_name: class_file.super_class_name,
            interface_names: class_file.interface_names,
        });
        //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
        let class_ref = unsafe {
            let class_ptr: *const Class<'_> = class_ref;
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

        let mut area = MethodArea::default();

        let file_system_path = FileSystemClassPath::new("./resources").unwrap();
        area.add_class_path(Box::new(file_system_path));
        let rt_jar_path = JarFileClassPath::new("./resources/rt.jar").unwrap();

        area.add_class_path(Box::new(rt_jar_path));
        let result = area.load_class("FieldTest").unwrap();

        assert!(matches!(result.status, ClassStatus::Loaded));
        assert_eq!(2, area.num_of_classes());

        let main_method = result
            .get_method_by_checking_super("main", "([Ljava/lang/String;)V")
            .unwrap();

        assert_eq!(main_method.name, "main");

        let option = area.get_mut(result);
        assert!(option.is_some());

        let system_class = area.load_class("java/lang/System").unwrap();
        println!("{}", system_class)
    }
}
