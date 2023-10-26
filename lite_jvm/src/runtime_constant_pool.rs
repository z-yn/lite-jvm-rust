use crate::jvm_exceptions::{Exception, Result};
use class_file_reader::class_file_error::ClassFileError;
use class_file_reader::constant_pool::{
    ConstantPool, ConstantPoolEntry, ConstantPoolPhysicalEntry,
};
use std::fmt::{Display, Formatter};

//https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-5.html#jvms-5.4.3.5
#[derive(Debug, PartialEq, Eq)]
pub enum MethodHandlerKind {
    GetField,
    GetStatic,
    PutField,
    PutStatic,
    InvokeVirtual,
    InvokeStatic,
    InvokeSpecial,
    NewInvokeSpecial,
    InvokeInterface,
}

impl MethodHandlerKind {
    pub fn new(kind: u8) -> Result<MethodHandlerKind> {
        match kind {
            1 => Ok(MethodHandlerKind::GetField),
            2 => Ok(MethodHandlerKind::GetStatic),
            3 => Ok(MethodHandlerKind::PutField),
            4 => Ok(MethodHandlerKind::PutStatic),
            5 => Ok(MethodHandlerKind::InvokeVirtual),
            6 => Ok(MethodHandlerKind::InvokeSpecial),
            7 => Ok(MethodHandlerKind::NewInvokeSpecial),
            8 => Ok(MethodHandlerKind::InvokeInterface),
            _ => Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidMethodHandlerKind(kind),
            ))),
        }
    }
}
impl Display for MethodHandlerKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodHandlerKind::GetField => write!(f, "getfield C.f:T"),
            MethodHandlerKind::GetStatic => write!(f, "getstatic C.f:T"),
            MethodHandlerKind::PutField => write!(f, "putfield C.f:T"),
            MethodHandlerKind::PutStatic => write!(f, "putstatic C.f:T"),
            MethodHandlerKind::InvokeVirtual => write!(f, "invokevirtual C.m:(A*)T"),
            MethodHandlerKind::InvokeStatic => write!(f, "invokestatic C.m:(A*)T"),
            MethodHandlerKind::InvokeSpecial => write!(f, "invokespecial C.m:(A*)T"),
            MethodHandlerKind::NewInvokeSpecial => {
                write!(f, "new C; dup; invokespecial C.<init>:(A*)V")
            }
            MethodHandlerKind::InvokeInterface => write!(f, "invokeinterface C.m:(A*)T"),
        }
    }
}
pub enum RuntimeConstantPoolEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    //解析ClassReference得到的是类名
    ClassReference(String),
    StringReference(String),
    // class_name,field_name,field_descriptor
    FieldReference(String, String, String),
    // class_name,method_name,method_descriptor
    MethodReference(String, String, String),
    // interface_name,method_name,method_descriptor
    InterfaceMethodReference(String, String, String),
    //name,descriptor
    NameAndTypeDescriptor(String, String),
    //type, FieldRef/MethodRef/InterfaceMethodRef
    MethodHandler(MethodHandlerKind, String, String, String),
    //descriptor
    MethodType(String),
    //bootstrap_method_attr_index, method_name,method_descriptor
    Dynamic(u16, String, String),
    //bootstrap_method_attr_index, method_name,method_descriptor
    InvokeDynamic(u16, String, String),
    Module(String),
    Package(String),
}

impl RuntimeConstantPoolEntry {
    fn get_utf8_string(cp: &ConstantPool, offset: &u16) -> Result<String> {
        cp.get_string(offset)
            .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))
    }

    fn get_class_name_string(cp: &ConstantPool, offset: &u16) -> Result<String> {
        let class_ref = cp
            .get(offset)
            .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))?;
        if let ConstantPoolEntry::ClassReference(name_index) = class_ref {
            Ok(Self::get_utf8_string(cp, name_index)?)
        } else {
            Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Not ClassRef ConstantValue".to_string()),
            )))
        }
    }
    fn get_name_and_type_string(cp: &ConstantPool, offset: &u16) -> Result<(String, String)> {
        let result = cp
            .get(offset)
            .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))?;
        if let ConstantPoolEntry::NameAndTypeDescriptor(name_idx, type_inx) = result {
            Ok((
                Self::get_utf8_string(cp, name_idx)?,
                Self::get_utf8_string(cp, type_inx)?,
            ))
        } else {
            Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Not NameAndType ConstantValue".to_string()),
            )))
        }
    }

    fn get_field_info_string(cp: &ConstantPool, offset: &u16) -> Result<(String, String, String)> {
        let result = cp
            .get(offset)
            .map_err(|e| Exception::ReadClassBytesError(Box::new(e)))?;
        match result {
            ConstantPoolEntry::MethodReference(class_index, name_and_type_index)
            | ConstantPoolEntry::FieldReference(class_index, name_and_type_index)
            | ConstantPoolEntry::InterfaceMethodReference(class_index, name_and_type_index) => {
                let class_name = Self::get_class_name_string(cp, class_index)?;
                let (name, descriptor) = Self::get_name_and_type_string(cp, name_and_type_index)?;
                Ok((class_name, name, descriptor))
            }
            _ => Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Not NameAndType ConstantValue".to_string()),
            ))),
        }
    }

    fn from(cp: &ConstantPool, entry: &ConstantPoolEntry) -> Result<RuntimeConstantPoolEntry> {
        let value = match entry {
            ConstantPoolEntry::Utf8(v) => RuntimeConstantPoolEntry::Utf8(String::from(v)),
            ConstantPoolEntry::Integer(v) => RuntimeConstantPoolEntry::Integer(*v),
            ConstantPoolEntry::Float(v) => RuntimeConstantPoolEntry::Float(*v),
            ConstantPoolEntry::Long(v) => RuntimeConstantPoolEntry::Long(*v),
            ConstantPoolEntry::Double(v) => RuntimeConstantPoolEntry::Double(*v),
            ConstantPoolEntry::ClassReference(offset) => {
                RuntimeConstantPoolEntry::ClassReference(Self::get_utf8_string(cp, offset)?)
            }
            ConstantPoolEntry::StringReference(offset) => {
                RuntimeConstantPoolEntry::StringReference(Self::get_utf8_string(cp, offset)?)
            }
            ConstantPoolEntry::FieldReference(class_name_idx, name_type_index) => {
                let class_name = Self::get_class_name_string(cp, class_name_idx)?;
                let (field_name, field_descriptor) =
                    Self::get_name_and_type_string(cp, name_type_index)?;
                RuntimeConstantPoolEntry::FieldReference(class_name, field_name, field_descriptor)
            }
            ConstantPoolEntry::MethodReference(class_name_idx, name_type_index) => {
                let class_name = Self::get_class_name_string(cp, class_name_idx)?;
                let (method_name, method_descriptor) =
                    Self::get_name_and_type_string(cp, name_type_index)?;
                RuntimeConstantPoolEntry::MethodReference(
                    class_name,
                    method_name,
                    method_descriptor,
                )
            }
            ConstantPoolEntry::InterfaceMethodReference(interface_name_idx, name_type_index) => {
                let interface_name = Self::get_class_name_string(cp, interface_name_idx)?;
                let (method_name, method_descriptor) =
                    Self::get_name_and_type_string(cp, name_type_index)?;
                RuntimeConstantPoolEntry::InterfaceMethodReference(
                    interface_name,
                    method_name,
                    method_descriptor,
                )
            }
            ConstantPoolEntry::NameAndTypeDescriptor(name_index, descriptor_index) => {
                let name = Self::get_utf8_string(cp, name_index)?;
                let descriptor = Self::get_utf8_string(cp, descriptor_index)?;
                RuntimeConstantPoolEntry::NameAndTypeDescriptor(name, descriptor)
            }
            ConstantPoolEntry::MethodHandler(reference_kind, reference_index) => {
                let kind = MethodHandlerKind::new(*reference_kind)?;
                let (class_or_interface_name, method_or_field_name, method_or_field_descriptor) =
                    Self::get_field_info_string(cp, reference_index)?;
                RuntimeConstantPoolEntry::MethodHandler(
                    kind,
                    class_or_interface_name,
                    method_or_field_name,
                    method_or_field_descriptor,
                )
            }
            ConstantPoolEntry::MethodType(descriptor_index) => {
                RuntimeConstantPoolEntry::MethodType(Self::get_utf8_string(cp, descriptor_index)?)
            }
            ConstantPoolEntry::Dynamic(bootstrap_method_attr_index, name_and_type_index) => {
                let (name, descriptor) = Self::get_name_and_type_string(cp, name_and_type_index)?;
                RuntimeConstantPoolEntry::Dynamic(*bootstrap_method_attr_index, name, descriptor)
            }
            ConstantPoolEntry::InvokeDynamic(bootstrap_method_attr_index, name_and_type_index) => {
                let (name, descriptor) = Self::get_name_and_type_string(cp, name_and_type_index)?;
                RuntimeConstantPoolEntry::InvokeDynamic(
                    *bootstrap_method_attr_index,
                    name,
                    descriptor,
                )
            }
            ConstantPoolEntry::Module(name_index) => {
                RuntimeConstantPoolEntry::Module(Self::get_utf8_string(cp, name_index)?)
            }
            ConstantPoolEntry::Package(name_index) => {
                RuntimeConstantPoolEntry::Package(Self::get_utf8_string(cp, name_index)?)
            }
        };
        Ok(value)
    }
}

pub enum RuntimeConstantPoolPhysicalEntry {
    Entry(RuntimeConstantPoolEntry),
    PlaceHolder,
}

/// 运行时常量池
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-5.html#jvms-5.1
/// 讲类常量池转换成运行时常量池。解析掉所有的引用，方便查找和使用。
pub struct RuntimeConstantPool {
    entries: Vec<RuntimeConstantPoolPhysicalEntry>,
}

impl RuntimeConstantPool {
    fn new() -> RuntimeConstantPool {
        RuntimeConstantPool {
            entries: Vec::new(),
        }
    }
    pub fn get_string(&self, index: u16) -> Result<String> {
        if let RuntimeConstantPoolEntry::StringReference(class_name) = self.get(index)? {
            Ok(class_name.clone())
        } else {
            Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Should Be StringRef".to_string()),
            )))
        }
    }
    pub fn get_utf8_string(&self, index: u16) -> Result<String> {
        if let RuntimeConstantPoolEntry::Utf8(class_name) = self.get(index)? {
            Ok(class_name.clone())
        } else {
            Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Should Be Utf8".to_string()),
            )))
        }
    }

    pub fn get_class_name(&self, index: u16) -> Result<&str> {
        if let RuntimeConstantPoolEntry::ClassReference(class_name) = self.get(index)? {
            Ok(class_name)
        } else {
            Err(Exception::ReadClassBytesError(Box::new(
                ClassFileError::InvalidClassData("Should Be ClassRef".to_string()),
            )))
        }
    }
    pub(crate) fn get(&self, index: u16) -> Result<&RuntimeConstantPoolEntry> {
        let offset = (index - 1) as usize;
        if self.entries.len() >= offset {
            if let RuntimeConstantPoolPhysicalEntry::Entry(entry) = &self.entries[offset] {
                return Ok(entry);
            }
        }
        Err(Exception::ReadClassBytesError(Box::new(
            ClassFileError::InvalidConstantPoolIndexError(index),
        )))
    }
    pub fn from(cp: &ConstantPool) -> Result<RuntimeConstantPool> {
        let mut runtime_cp = Self::new();
        for entry in &cp.entries {
            let runtime_entry = match entry {
                ConstantPoolPhysicalEntry::Entry(e) => {
                    RuntimeConstantPoolPhysicalEntry::Entry(RuntimeConstantPoolEntry::from(&cp, e)?)
                }
                ConstantPoolPhysicalEntry::PlaceHolder => {
                    RuntimeConstantPoolPhysicalEntry::PlaceHolder
                }
            };
            runtime_cp.entries.push(runtime_entry);
        }
        Ok(runtime_cp)
    }
}
