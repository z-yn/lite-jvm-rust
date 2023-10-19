use crate::attribute_info::{AttributeInfo, AttributeType};
use crate::cesu8_byte_buffer::ByteBuffer;
use crate::class_file::{ClassAccessFlags, ClassFile};
use crate::class_file_error::{ClassFileError, Result};
use crate::class_file_version::ClassFileVersion;
use crate::constant_pool::{ConstantPool, ConstantPoolEntry};
use crate::field_info::{FieldAccessFlags, FieldInfo};
use crate::method_info::{MethodAccessFlags, MethodInfo};

/// Reads a class from a byte slice.
/// ClassFile {
///     u4             magic;
///     u2             minor_version;
///     u2             major_version;
///     u2             constant_pool_count;
///     cp_info        constant_pool[constant_pool_count-1];
///     u2             access_flags;
///     u2             this_class;
///     u2             super_class;
///     u2             interfaces_count;
///     u2             interfaces[interfaces_count];
///     u2             fields_count;
///     field_info     fields[fields_count];
///     u2             methods_count;
///     method_info    methods[methods_count];
///     u2             attributes_count;
///     attribute_info attributes[attributes_count];
/// }
///
pub fn read_buffer(buf: &[u8]) -> Result<ClassFile> {
    let mut buffer = ByteBuffer::new(buf);
    check_magic_number(&mut buffer)?;
    let version = read_version(&mut buffer)?;
    let constant_pool = read_const_pool(&mut buffer)?;
    let access_flags = read_access_flag(&mut buffer)?;
    let this_class = buffer.read_u16()?;
    let this_class_name = constant_pool.get_class_name(&this_class)?;

    let super_class = buffer.read_u16()?;
    let super_class_name = constant_pool.try_get_class_name(&super_class);

    let interface_names = read_interfaces(&mut buffer, &constant_pool)?;
    let field_info = read_field_info(&mut buffer, &constant_pool)?;
    let method_info = read_method_info(&mut buffer, &constant_pool)?;
    let attribute_info = read_attribute_info(&mut buffer, &constant_pool)?;
    //此时应该读取完所有数据
    assert!(!buffer.has_more_data());
    Ok(ClassFile {
        version,
        constant_pool,
        access_flags,
        this_class_name,
        super_class_name,
        interface_names,
        field_info,
        method_info,
        attribute_info,
    })
}

fn check_magic_number(buffer: &mut ByteBuffer) -> Result<()> {
    match buffer.read_u32() {
        Ok(0xCAFEBABE) => Ok(()),
        Ok(n) => Err(ClassFileError::InvalidClassData(String::from(format!(
            "invalid magic number: {n}"
        )))),
        Err(err) => Err(err.into()),
    }
}
fn read_version(buffer: &mut ByteBuffer) -> Result<ClassFileVersion> {
    let minor_version = buffer.read_u16()?;
    let major_version = buffer.read_u16()?;
    ClassFileVersion::new(major_version, minor_version)
}
fn read_const_pool(buffer: &mut ByteBuffer) -> Result<ConstantPool> {
    let mut constant_pool = ConstantPool::new();
    let constant_pool_count = buffer.read_u16()? as usize;
    while constant_pool.len() < constant_pool_count - 1 {
        constant_pool.add(ConstantPoolEntry::read_from_bytes(buffer)?);
    }
    Ok(constant_pool)
}

fn read_access_flag(buffer: &mut ByteBuffer) -> Result<ClassAccessFlags> {
    let access_flag = buffer.read_u16()?;
    match ClassAccessFlags::from_bits(access_flag) {
        Some(flags) => Ok(flags),
        None => Err(ClassFileError::InvalidClassData(format!(
            "invalid class flags: {access_flag}"
        ))),
    }
}

fn read_interfaces(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<Vec<String>> {
    let interfaces_count = buffer.read_u16()? as usize;
    let mut result = Vec::new();
    for _ in 0..interfaces_count {
        let offset = buffer.read_u16()?;
        result.push(cp.get_class_name(&offset)?);
    }
    Ok(result)
}

fn read_field_info(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<Vec<FieldInfo>> {
    let field_count = buffer.read_u16()? as usize;
    (0..field_count)
        .map(|_| read_one_field(buffer, cp))
        .collect()
}
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.5
/// 结构如下
/// ```c
/// field_info {
///     u2             access_flags;
///     u2             name_index;
///     u2             descriptor_index;
///     u2             attributes_count;
///     attribute_info attributes[attributes_count];
/// }
/// ```
///

fn read_one_field(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<FieldInfo> {
    let access_flag = buffer.read_u16()?;

    let name_index = buffer.read_u16()?;
    let access_flags = match FieldAccessFlags::from_bits(access_flag) {
        Some(flags) => flags,
        None => {
            return Err(ClassFileError::InvalidClassData(format!(
                "invalid field flags: {access_flag}"
            )))
        }
    };
    let name = cp.get_string(&name_index)?;
    let descriptor_index = buffer.read_u16()?;
    let descriptor = cp.get_string(&descriptor_index)?;
    let attributes = read_attribute_info(buffer, cp)?;
    Ok(FieldInfo {
        access_flags,
        name,
        descriptor,
        attributes,
    })
}

fn read_method_info(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<Vec<MethodInfo>> {
    let methods_count = buffer.read_u16()? as usize;
    (0..methods_count)
        .map(|_| read_one_method(buffer, cp))
        .collect()
}
/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.6
///```c
///method_info {
///     u2             access_flags;
///     u2             name_index;
///     u2             descriptor_index;
///     u2             attributes_count;
///     attribute_info attributes[attributes_count];
/// }
/// ```
///
fn read_one_method(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<MethodInfo> {
    let access_flag = buffer.read_u16()?;
    let access_flags = match MethodAccessFlags::from_bits(access_flag) {
        Some(flags) => flags,
        None => {
            return Err(ClassFileError::InvalidClassData(format!(
                "invalid field flags: {access_flag}"
            )))
        }
    };
    let name_index = buffer.read_u16()?;
    let name = cp.get_string(&name_index)?;
    let descriptor_index = buffer.read_u16()?;
    let descriptor = cp.get_string(&descriptor_index)?;
    let attributes = read_attribute_info(buffer, cp)?;
    Ok(MethodInfo {
        access_flags,
        name,
        descriptor,
        attributes,
    })
}

fn read_attribute_info(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<Vec<AttributeInfo>> {
    let attribute_count = buffer.read_u16()? as usize;
    (0..attribute_count)
        .map(|_| read_one_attribute(buffer, cp))
        .collect()
}

/// https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7
/// ```c
///attribute_info {
///     u2 attribute_name_index;
///     u4 attribute_length;
///     u1 info[attribute_length];
/// }
/// ```
fn read_one_attribute(buffer: &mut ByteBuffer, cp: &ConstantPool) -> Result<AttributeInfo> {
    let attribute_name_index = buffer.read_u16()?;

    let name = if let ConstantPoolEntry::Utf8(value) = cp.get(&attribute_name_index)? {
        AttributeType::by_name(value)
    } else {
        return Err(ClassFileError::InvalidClassData(format!(
            "Should be utf8 String at {attribute_name_index}"
        )));
    };
    let attribute_length = buffer.read_u32()? as usize;
    let bytes = buffer.read_bytes(attribute_length)?;
    Ok(AttributeInfo {
        name,
        info: Vec::from(bytes),
    })
}
