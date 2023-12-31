use crate::jvm_error::VmExecResult;
use crate::runtime_attribute_info::{get_attr_as_constant_value, ConstantValueAttribute};
use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::attribute_info::{AttributeInfo, AttributeType};
use class_file_reader::field_info::{FieldAccessFlags, FieldInfo};

pub struct RuntimeFieldInfo {
    pub access_flags: FieldAccessFlags,
    pub name: String,
    pub descriptor: String,
    pub constant_value: Option<ConstantValueAttribute>,
    //内存中的索引值，从1开始。0表示未设置索引,即静态方法位置
    pub offset: usize,
    pub attributes: Vec<AttributeInfo>,
}

impl RuntimeFieldInfo {
    pub fn is_static(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::STATIC)
    }
    pub fn from(field_info: FieldInfo, cp: &RuntimeConstantPool) -> VmExecResult<RuntimeFieldInfo> {
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
            offset: 0,
            attributes: field_info.attributes,
        })
    }
}
