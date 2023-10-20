use crate::loaded_class::ClassRef;

///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-2.html#jvms-2.2
///
/// 用来表示放到内存中的数据

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Value {
    #[default]
    Uninitialized,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Char(u16),
    Float(f32),
    Double(f64),
    ReturnAddress(u16),
    Null,
}

pub struct Object {}
