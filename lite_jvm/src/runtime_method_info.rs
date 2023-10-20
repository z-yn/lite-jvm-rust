use crate::jvm_exceptions::Result;
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
    pub fn from(method_info: MethodInfo, cp: &RuntimeConstantPool) -> Result<RuntimeMethodInfo> {
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
