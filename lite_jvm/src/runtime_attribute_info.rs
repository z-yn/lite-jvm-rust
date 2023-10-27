use crate::jvm_error::{VmError, VmExecResult};
use crate::runtime_constant_pool::{RuntimeConstantPool, RuntimeConstantPoolEntry};
use class_file_reader::cesu8_byte_buffer::ByteBuffer;
use class_file_reader::class_file_error;
use indexmap::IndexMap;
use std::fmt::{Display, Formatter};

///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.2
pub enum ConstantValueAttribute {
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(String),
}

pub(crate) fn get_attr_as_constant_value(
    value: &Vec<u8>,
    cp: &RuntimeConstantPool,
) -> VmExecResult<ConstantValueAttribute> {
    assert_eq!(2, value.len());
    let bytes = &value[..];
    let const_pool_index = u16::from_be_bytes(bytes.try_into().unwrap());
    match cp.get(const_pool_index)? {
        RuntimeConstantPoolEntry::Integer(v) => Ok(ConstantValueAttribute::Int(v.clone())),
        RuntimeConstantPoolEntry::Float(v) => Ok(ConstantValueAttribute::Float(v.clone())),
        RuntimeConstantPoolEntry::Long(v) => Ok(ConstantValueAttribute::Long(v.clone())),
        RuntimeConstantPoolEntry::Double(v) => Ok(ConstantValueAttribute::Double(v.clone())),
        RuntimeConstantPoolEntry::StringReference(v) => {
            Ok(ConstantValueAttribute::String(v.clone()))
        }
        _ => Err(VmError::InvalidAttribute("".to_string())).unwrap(),
    }
}

impl Display for ConstantValueAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValueAttribute::Int(v) => write!(f, "ConstantValue=>Int:{v}"),
            ConstantValueAttribute::Float(v) => write!(f, "ConstantValue=>Float:{v}"),
            ConstantValueAttribute::Long(v) => write!(f, "ConstantValue=>Long:{v}"),
            ConstantValueAttribute::Double(v) => write!(f, "ConstantValue=>Double:{v}"),
            ConstantValueAttribute::String(v) => write!(f, "ConstantValue=>String:{v}"),
        }
    }
}
//

///
/// ```c
/// Code_attribute {
///     u2 attribute_name_index;
///     u4 attribute_length;
///
///     u2 max_stack;
///     u2 max_locals;
///     u4 code_length;
///     u1 code[code_length];
///     u2 exception_table_length;
///     {   u2 start_pc;
///         u2 end_pc;
///         u2 handler_pc;
///         u2 catch_type;
///     } exception_table[exception_table_length];
///     u2 attributes_count;
///     attribute_info attributes[attributes_count];
/// }
/// ```
///
/// LineNumberTable	Code	45.3
// LocalVariableTable	Code	45.3
// LocalVariableTypeTable	Code	49.0
// StackMapTable	Code
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionTable>,
    //start_pc -> line number
    pub line_number_table: IndexMap<u16, u16>,
    pub local_variable_table: IndexMap<u16, LocalVariableTable>,
    pub local_variable_type_table: IndexMap<u16, LocalVariableTypeTable>,
}

pub struct ExceptionTable {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: Option<String>,
}

pub struct LocalVariableTable {
    pub start_pc: u16,
    pub length: u16,
    pub name: String,
    pub descriptor: String,
}

pub struct LocalVariableTypeTable {
    pub start_pc: u16,
    pub length: u16,
    pub name: String,
    pub signature: String,
}
///
/// TODO StackMapTable有点复杂，英文文档看到脑壳疼。后面再实现
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.4
pub struct StackMapTable {}

fn read_code_bytes(
    value: &Vec<u8>,
    cp: &RuntimeConstantPool,
) -> class_file_error::Result<CodeAttribute> {
    let mut buffer = ByteBuffer::new(value);
    let max_stack = buffer.read_u16()?;
    let max_locals = buffer.read_u16()?;
    let code_length = buffer.read_u32()?;
    let code = buffer.read_bytes(code_length as usize)?;
    let exception_table_length = buffer.read_u16()?;
    let mut exception_table = Vec::new();

    for _ in 0..exception_table_length {
        let start_pc = buffer.read_u16()?;
        let end_pc = buffer.read_u16()?;
        let handler_pc = buffer.read_u16()?;
        let catch_type_index = buffer.read_u16()?;
        let catch_type = if catch_type_index == 0 {
            None
        } else {
            Some(cp.get_class_name(catch_type_index).unwrap().to_string())
        };
        exception_table.push(ExceptionTable {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        });
    }
    let attributes_count = buffer.read_u16()?;
    let mut line_number_table = IndexMap::new();
    let mut local_variable_table = IndexMap::new();
    let mut local_variable_type_table = IndexMap::new();
    for _ in 0..attributes_count {
        let attribute_name_index = buffer.read_u16()?;
        let attribute_length = buffer.read_u32()?;
        let attribute_bytes = buffer.read_bytes(attribute_length as usize)?;
        let attribute_name = cp.get_utf8_string(attribute_name_index).unwrap();
        if attribute_name == "LineNumberTable" {
            let mut line_number_reader = ByteBuffer::new(attribute_bytes);
            let line_number_table_length = line_number_reader.read_u16()?;
            for _ in 0..line_number_table_length {
                let start_pc = line_number_reader.read_u16()?;
                let line_number = line_number_reader.read_u16()?;
                line_number_table.insert(start_pc, line_number);
            }
        } else if attribute_name == "LocalVariableTable" {
            let mut local_variable_table_reader = ByteBuffer::new(attribute_bytes);
            let local_variable_table_length = local_variable_table_reader.read_u16()?;
            for _ in 0..local_variable_table_length {
                let start_pc = local_variable_table_reader.read_u16()?;
                let length = local_variable_table_reader.read_u16()?;
                let name_index = local_variable_table_reader.read_u16()?;
                let descriptor_index = local_variable_table_reader.read_u16()?;
                let index = local_variable_table_reader.read_u16()?;
                local_variable_table.insert(
                    index,
                    LocalVariableTable {
                        start_pc,
                        length,
                        name: cp.get_utf8_string(name_index).unwrap(),
                        descriptor: cp.get_utf8_string(descriptor_index).unwrap(),
                    },
                );
            }
        } else if attribute_name == "LocalVariableTypeTable" {
            let mut local_variable_type_table_reader = ByteBuffer::new(attribute_bytes);
            let local_variable_type_table_length = local_variable_type_table_reader.read_u16()?;
            for _ in 0..local_variable_type_table_length {
                let start_pc = local_variable_type_table_reader.read_u16()?;
                let length = local_variable_type_table_reader.read_u16()?;
                let name_index = local_variable_type_table_reader.read_u16()?;
                let signature_index = local_variable_type_table_reader.read_u16()?;
                let index = local_variable_type_table_reader.read_u16()?;
                local_variable_type_table.insert(
                    index,
                    LocalVariableTypeTable {
                        start_pc,
                        length,
                        name: cp.get_utf8_string(name_index).unwrap(),
                        signature: cp.get_utf8_string(signature_index).unwrap(),
                    },
                );
            }
        }
    }
    Ok(CodeAttribute {
        max_stack,
        max_locals,
        code: Vec::from(code),
        exception_table,
        line_number_table,
        local_variable_table,
        local_variable_type_table,
    })
}
pub(crate) fn get_attr_as_code(
    value: &Vec<u8>,
    cp: &RuntimeConstantPool,
) -> VmExecResult<CodeAttribute> {
    read_code_bytes(value, cp).map_err(|e| VmError::ReadClassBytesError(e.to_string()))
}

pub(crate) fn get_attr_as_exception(bytes: &Vec<u8>, cp: &RuntimeConstantPool) -> Vec<String> {
    let mut buffer = ByteBuffer::new(bytes);
    let number_of_exceptions = buffer.read_u16().unwrap();
    (0..number_of_exceptions)
        .map(|_| {
            let exception_index = buffer.read_u16().unwrap();
            cp.get_class_name(exception_index).unwrap().to_string()
        })
        .collect()
}

//BootstrapMethods
