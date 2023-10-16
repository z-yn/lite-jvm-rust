use crate::attribute_info::AttributeInfo;
use bitflags::bitflags;
bitflags! {
    /// ## Class flags
    /// [jvms refer](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.6)
    pub struct MethodAccessFlags: u16 {
        const PUBLIC = 0x0001;
        const PRIVATE = 0x0002;
        const PROTECTED = 0x0004;
        const STATIC = 0x0008;
        const FINAL = 0x0010;
        const SYNCHRONIZED = 0x0020;
        const BRIDGE = 0x0040;
        const VARARGS = 0x0080;
        const NATIVE = 0x1000;
        const ABSTRACT = 0x4000;
        const STRICT = 0x0800;
        const SYNTHETIC = 0x1000;
    }
}

impl Default for MethodAccessFlags {
    fn default() -> MethodAccessFlags {
        MethodAccessFlags::empty()
    }
}
pub struct MethodInfo {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: String,
    pub attributes: Vec<AttributeInfo>,
}
