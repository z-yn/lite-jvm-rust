use crate::bootstrap_class_loader::LoadClassResult::{AlreadyLoaded, NewLoaded};
use crate::class_finder::{ClassFinder, ClassPath};
use crate::jvm_exceptions::Exception::ClassNotFoundException;
use crate::jvm_exceptions::{Exception, Result};
use crate::loaded_class::{load_class, ClassRef, LoadedClass};
use std::collections::HashMap;

///实现BootstrapClassLoader。
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-5.html#jvms-5.3.1
/// 1.先确定是否已经加载。如果已经加载，则没有类加载/创建发生
/// 2. Java虚拟机将参数N传递给引导类装入器上的方法调用。返回类实例。（此处没有限制，因为是平台相关的）
///
/// 根据ClassPath查询
///
///
/// 类加载器实现原则：
/// 1. 给定相同的名称，一个好的类装入器应该总是返回相同的class对象。
/// 2. 如果类装入器L1将类C的装入委托给另一个装入器L2，那么以下四种场景的类T，L1和L2应该返回相同的class对象
///    - 对于作为C的直接super class / super interfaces
///    - 作为C中字段的类型
///    - 作为C语言中方法或构造函数的形式参数的类型，
///    - 作为C中方法的返回类型
/// 3. 对于自定义类加载器，如果预加载/批量加载一批相关时，需要跟没有预加载/批量加载的相同情况下抛出异常
///
///
pub enum LoadClassResult<'a> {
    NewLoaded(LoadedClass<'a>),
    AlreadyLoaded(ClassRef<'a>),
}
pub trait ClassLoader<'a> {
    fn load_class(&mut self, name: &str) -> Result<LoadClassResult<'a>>;

    fn registry_class(&mut self, name: &str, class: LoadedClass<'a>) -> Result<ClassRef<'a>>;
}

pub struct BootstrapClassLoader<'a> {
    class_finder: ClassFinder,
    loaded_class: HashMap<String, LoadedClass<'a>>,
}

impl<'a> BootstrapClassLoader<'a> {
    pub(crate) fn new() -> BootstrapClassLoader<'a> {
        BootstrapClassLoader {
            class_finder: ClassFinder::new(),
            loaded_class: HashMap::new(),
        }
    }

    pub(crate) fn add_class_path(&mut self, path: Box<dyn ClassPath>) {
        self.class_finder.class_paths.push(path);
    }
}

impl<'a> ClassLoader<'a> for BootstrapClassLoader<'a> {
    fn load_class(&mut self, name: &str) -> Result<LoadClassResult<'a>> {
        match self.loaded_class.get(name) {
            Some(v) => Ok(AlreadyLoaded(v)),
            None => {
                let result = self.class_finder.find_class(name)?;
                Ok(NewLoaded(load_class(result)))
            }
        }
    }

    fn registry_class(&mut self, class_name: &str, class: LoadedClass<'a>) -> Result<ClassRef<'a>> {
        self.loaded_class.insert(class.name.clone(), class);
        if let AlreadyLoaded(v) = self.load_class(class_name)? {
            Ok(v)
        } else {
            Err(Exception::ClassNotFoundException("".to_string()))
        }
    }
}
