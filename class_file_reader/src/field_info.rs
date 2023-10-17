use crate::attribute_info::AttributeInfo;
use bitflags::bitflags;

bitflags! {
    /// ## Class flags
    /// [jvms refer](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.1-200-E.1)
    pub struct FieldAccessFlags: u16 {
        const PUBLIC = 0x0001;
        const PRIVATE = 0x0002;
        const PROTECTED = 0x0004;
        const STATIC = 0x0008;
        const FINAL = 0x0010;
        const VOLATILE = 0x0040;
        const TRANSIENT = 0x0080;
        const SYNTHETIC = 0x1000;
        const ENUM = 0x4000;
    }
}

impl Default for FieldAccessFlags {
    fn default() -> FieldAccessFlags {
        FieldAccessFlags::empty()
    }
}

/// ## 字段信息
/// [jvms refer](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.5)
/// ``` c
/// field_info {
///     u2             access_flags;
///     u2             name_index;
///     u2             descriptor_index;
///     u2             attributes_count;
///     attribute_info attributes[attributes_count];
/// }
/// ```
pub struct FieldInfo {
    pub access_flags: FieldAccessFlags,
    //name_index ->解析的时候可以直接读出来
    pub name: String,
    //descriptor_index
    pub descriptor: String,
    pub attributes: Vec<AttributeInfo>,
}
