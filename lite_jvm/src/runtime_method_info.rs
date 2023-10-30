use crate::jvm_error::VmExecResult;
use crate::runtime_attribute_info::{get_attr_as_code, get_attr_as_exception, CodeAttribute};
use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::attribute_info::AttributeType;
use class_file_reader::method_info::{MethodAccessFlags, MethodInfo};

pub struct RuntimeMethodInfo {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: String,
    //除了native方法应该都有code属性
    pub code: Option<CodeAttribute>,
    pub exception: Vec<String>,
}

///Code	method_info	45.3
// Exceptions	method_info	45.3
// RuntimeVisibleParameterAnnotations, RuntimeInvisibleParameterAnnotations	method_info	49.0
// AnnotationDefault	method_info	49.0
// MethodParameters	method_info	52.0
impl RuntimeMethodInfo {
    pub(crate) fn is_native(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::NATIVE)
    }
    pub(crate) fn is_static(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STATIC)
    }
    pub fn from(
        method_info: MethodInfo,
        cp: &RuntimeConstantPool,
    ) -> VmExecResult<RuntimeMethodInfo> {
        let mut code = None;
        let mut exception = Vec::new();
        for attr in &method_info.attributes {
            match attr.name {
                AttributeType::Code => code = Some(get_attr_as_code(&attr.info, cp)?),

                AttributeType::Exceptions => exception = get_attr_as_exception(&attr.info, cp),
                // AttributeType::RuntimeVisibleParameterAnnotations => {}
                // AttributeType::RuntimeInvisibleParameterAnnotations => {}
                _ => {}
            }
        }
        Ok(RuntimeMethodInfo {
            access_flags: method_info.access_flags,
            name: method_info.name,
            descriptor: method_info.descriptor,
            code,
            exception,
        })
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct MethodKey<'a>(&'a str, &'a str);

impl<'a> MethodKey<'a> {
    pub fn new(name: &'a str, descriptor: &'a str) -> MethodKey<'a> {
        MethodKey(name, descriptor)
    }

    pub fn by_method(method: &RuntimeMethodInfo) -> MethodKey<'a> {
        let name = unsafe {
            let str_ptr: *const str = method.name.as_str();
            &*str_ptr
        };
        let descriptor = unsafe {
            let str_ptr: *const str = method.descriptor.as_str();
            &*str_ptr
        };
        MethodKey(name, descriptor)
    }
}
