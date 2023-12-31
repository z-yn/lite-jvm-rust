use crate::jvm_error::{VmError, VmExecResult};
use crate::runtime_attribute_info::BootstrapMethod;
use crate::runtime_constant_pool::RuntimeConstantPool;
use crate::runtime_field_info::RuntimeFieldInfo;
use crate::runtime_method_info::{MethodKey, RuntimeMethodInfo};
use class_file_reader::class_file::ClassAccessFlags;
use class_file_reader::class_file_version::ClassFileVersion;
use indexmap::IndexMap;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq, PartialEq)]
pub enum ClassStatus {
    Loading,
    Loaded,
    Linking,
    Linked,
    Initializing,
    Initialized,
}

/// 表示加载的类，加载后该类会经过->链接->初始化过程最终加载完成。
///
pub struct Class<'a> {
    pub version: ClassFileVersion,
    pub status: ClassStatus,
    pub name: String,
    //常量池解析
    pub constant_pool: RuntimeConstantPool,
    pub access_flags: ClassAccessFlags,
    //超类解析
    pub super_class: Option<ClassRef<'a>>,
    //接口解析
    pub interfaces: IndexMap<&'a str, ClassRef<'a>>,
    //字段解析
    pub fields: IndexMap<&'a str, RuntimeFieldInfo>,
    //方法解析
    pub methods: IndexMap<MethodKey<'a>, RuntimeMethodInfo>,

    pub super_class_name: Option<String>,
    pub interface_names: Vec<String>,

    pub source_file: Option<String>,

    pub total_num_of_fields: usize,

    pub bootstrap_method: Vec<BootstrapMethod>,
}

impl<'a> Class<'a> {
    pub fn get_field_by_name(&'a self, name: &str) -> VmExecResult<FieldRef<'a>> {
        if let Some(field) = self.fields.get(name) {
            return Ok(field);
        }
        if let Some(super_class) = self.super_class {
            return super_class.get_field_by_name(name);
        }
        Err(VmError::FieldNotFoundException(name.to_string()))
    }
    pub(crate) fn get_field(&self, offset: usize) -> VmExecResult<FieldRef<'a>> {
        assert!(offset < self.total_num_of_fields);
        let super_class_offset = if let Some(class_ref) = self.super_class {
            if offset < class_ref.total_num_of_fields {
                return Ok(class_ref.fields.get_index(offset).unwrap().1);
            }
            class_ref.total_num_of_fields
        } else {
            0
        };
        let field = self
            .fields
            .get_index(offset - super_class_offset)
            .expect("")
            .1;
        //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
        let method_ref = unsafe {
            let const_ptr: *const RuntimeFieldInfo = field;
            &*const_ptr
        };
        Ok(method_ref)
    }

    pub fn is_interface(&self) -> bool {
        self.access_flags.contains(ClassAccessFlags::INTERFACE)
    }

    pub fn is_abstract(&self) -> bool {
        self.access_flags.contains(ClassAccessFlags::ABSTRACT)
    }

    pub(crate) fn is_subclass_of(&self, class_name: &str) -> bool {
        if self.name == class_name {
            return true;
        }
        if self.interfaces.get(class_name).is_some() {
            return true;
        }
        if let Some(super_class) = self.super_class {
            if super_class.is_subclass_of(class_name) {
                return true;
            }
        }
        false
    }

    pub fn get_method(
        &'a self,
        method_name: &str,
        descriptor: &str,
    ) -> VmExecResult<MethodRef<'a>> {
        if let Some(method) = self.methods.get(&MethodKey::new(method_name, descriptor)) {
            //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
            let method_ref = unsafe {
                let const_ptr: *const RuntimeMethodInfo = method;
                &*const_ptr
            };
            Ok(method_ref)
        } else {
            Err(VmError::MethodNotFoundException(
                method_name.to_string(),
                descriptor.to_string(),
            ))
        }
    }
    pub fn get_method_by_checking_super(
        &'a self,
        method_name: &str,
        descriptor: &str,
    ) -> VmExecResult<(ClassRef<'a>, MethodRef<'a>)> {
        if let Some(method) = self.methods.get(&MethodKey::new(method_name, descriptor)) {
            //self的声明周期要大于classRef<'a>,实用unsafe 使得编译器能够编译
            let method_ref = unsafe {
                let const_ptr: *const RuntimeMethodInfo = method;
                &*const_ptr
            };
            return Ok((self, method_ref));
        }

        //查找父类
        if let Some(supper_class) = &self.super_class {
            let by_super_class = supper_class.get_method_by_checking_super(method_name, descriptor);
            if by_super_class.is_ok() {
                return by_super_class;
            }
        }
        //查找接口
        for (_, interface) in &self.interfaces {
            let by_interface = interface.get_method_by_checking_super(method_name, descriptor);
            if by_interface.is_ok() {
                return by_interface;
            }
        }
        Err(VmError::MethodNotFoundException(
            method_name.to_string(),
            descriptor.to_string(),
        ))
    }
}

pub type ClassRef<'a> = &'a Class<'a>;

impl<'a> Hash for Class<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl<'a> PartialEq<Self> for Class<'a> {
    fn eq(&self, other: &Self) -> bool {
        let v1 = self as *const Class;
        let v2 = other as *const Class;
        v1 == v2
    }
}

impl<'a> Eq for Class<'a> {}

pub type MethodRef<'a> = &'a RuntimeMethodInfo;

pub type FieldRef<'a> = &'a RuntimeFieldInfo;

impl<'a> Display for Class<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.access_flags.contains(ClassAccessFlags::PUBLIC) {
            write!(f, "public ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::FINAL) {
            write!(f, "final ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::ABSTRACT) {
            write!(f, "abstract ")?;
        }
        if self.access_flags.contains(ClassAccessFlags::INTERFACE) {
            write!(f, "interface {}", self.name)?;
        } else if self.access_flags.contains(ClassAccessFlags::ENUM) {
            write!(f, "enum {}", self.name)?;
        } else {
            write!(f, "class {}", self.name)?;
        }

        write!(f, "flags: ({:#06x}) ", self.access_flags.bits())?;
        for bitflags in self.access_flags.iter() {
            match bitflags {
                ClassAccessFlags::PUBLIC => write!(f, "ACC_PUBLIC")?,
                ClassAccessFlags::SUPER => write!(f, "ACC_SUPER")?,
                _ => {}
            }
        }
        writeln!(f, "this_class: {}", self.name)?;
        if let Some(super_class) = &self.super_class_name {
            writeln!(f, "super_class: {}", super_class)?;
        }
        writeln!(
            f,
            "interfaces: {}, fields: {}, method: {}",
            self.interface_names.len(),
            self.fields.len(),
            self.methods.len(),
        )?;
        writeln!(f, "Constant pool:")?;
        writeln!(f, "{}", self.constant_pool)?;

        Ok(())
    }
}
