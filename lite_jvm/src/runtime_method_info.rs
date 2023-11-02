use crate::jvm_error::{VmError, VmExecResult};
use crate::reference_value::{PrimaryType, ValueType};
use crate::runtime_attribute_info::{get_attr_as_code, get_attr_as_exception, CodeAttribute};
use crate::runtime_constant_pool::RuntimeConstantPool;
use class_file_reader::attribute_info::AttributeType;
use class_file_reader::method_info::{MethodAccessFlags, MethodInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDescriptor {
    pub args: Vec<ValueType>,
    pub ret: ValueType,
}

impl MethodDescriptor {
    fn construct_primary_type(array_count: &mut usize, value_type: PrimaryType) -> ValueType {
        if *array_count > 0 {
            let primary_array = ValueType::PrimaryArray(value_type, *array_count);
            *array_count = 0;
            primary_array
        } else {
            ValueType::Primary(value_type)
        }
    }

    fn construct_object_type(mut array_count: &mut usize, value_type: Vec<char>) -> ValueType {
        let class_name = value_type.iter().collect();
        if *array_count > 0 {
            let primary_array = ValueType::ObjectArray(class_name, *array_count);
            *array_count = 0;
            primary_array
        } else {
            ValueType::Object(class_name)
        }
    }
    fn new(descriptor: &str) -> VmExecResult<MethodDescriptor> {
        assert!(descriptor.starts_with("("));
        let mut args = Vec::new();
        let mut reader = Vec::new();
        let mut array_count: usize = 0;

        for c in descriptor.chars() {
            if ')' == c || '(' == c {
                continue;
            }
            if reader.is_empty() {
                match c {
                    'V' => args.push(ValueType::Void),
                    'B' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Byte,
                    )),
                    'C' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Char,
                    )),
                    'D' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Double,
                    )),
                    'F' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Float,
                    )),
                    'I' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Int,
                    )),
                    'J' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Long,
                    )),
                    'S' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Short,
                    )),
                    'Z' => args.push(Self::construct_primary_type(
                        &mut array_count,
                        PrimaryType::Boolean,
                    )),
                    'L' => reader.push(c),
                    '[' => array_count += 1,
                    _ => return Err(VmError::ValueTypeMissMatch),
                }
            } else {
                match c {
                    ';' => {
                        args.push(Self::construct_object_type(&mut array_count, reader));
                        reader = Vec::new()
                    }
                    _ => reader.push(c),
                }
            }
        }

        let ret = args.pop().unwrap();
        Ok(MethodDescriptor { args, ret })
    }
}
pub struct RuntimeMethodInfo {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: String,
    pub descriptor_args_ret: MethodDescriptor,
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
    pub fn is_native(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::NATIVE)
    }
    pub fn is_static(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STATIC)
    }

    pub fn is_class_init_method(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STATIC) && self.name.as_str() == "<clinit>"
    }

    pub fn is_init_method(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STATIC) && self.name.as_str() == "<init>"
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
        let descriptor_args_ret = MethodDescriptor::new(&method_info.descriptor)?;
        Ok(RuntimeMethodInfo {
            access_flags: method_info.access_flags,
            name: method_info.name,
            descriptor: method_info.descriptor,
            descriptor_args_ret,
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
