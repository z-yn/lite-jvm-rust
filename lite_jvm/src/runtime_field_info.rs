use crate::jvm_exceptions::Result;
use crate::runtime_attribute_info::{get_attr_as_constant_value, ConstantValueAttribute};
use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::attribute_info::AttributeType;
use class_file_reader::field_info::{FieldAccessFlags, FieldInfo};

pub struct RuntimeFieldInfo {
    pub access_flags: FieldAccessFlags,
    pub name: String,
    pub descriptor: String,
    pub constant_value: Option<ConstantValueAttribute>,
}

impl RuntimeFieldInfo {
    pub fn from(field_info: FieldInfo, cp: &RuntimeConstantPool) -> Result<RuntimeFieldInfo> {
        let mut constant_value: Option<ConstantValueAttribute> = None;
        for attr in &field_info.attributes {
            if let AttributeType::ConstantValue = attr.name {
                constant_value = Some(get_attr_as_constant_value(&attr.info, cp)?)
            }
        }
        Ok(RuntimeFieldInfo {
            access_flags: field_info.access_flags,
            name: field_info.name,
            descriptor: field_info.descriptor,
            constant_value,
        })
    }
}
