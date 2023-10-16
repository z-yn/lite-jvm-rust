use std::fmt::{Display, Formatter};

/// ## 属性信息
/// 属性可以出现在，字段、方法，类中，是重要的扩展机制
/// [jvms-4.7](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7)
#[derive(Debug, Default, PartialEq)]
pub struct AttributeInfo {
    pub name: String,
    pub info: Vec<u8>,
}

pub trait Attribute: Display {}
///https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.2
pub enum ConstantValue {
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(String),
}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::Int(v) => write!(f, "ConstantValue=>Int:{v}"),
            ConstantValue::Float(v) => write!(f, "ConstantValue=>Float:{v}"),
            ConstantValue::Long(v) => write!(f, "ConstantValue=>Long:{v}"),
            ConstantValue::Double(v) => write!(f, "ConstantValue=>Double:{v}"),
            ConstantValue::String(v) => write!(f, "ConstantValue=>String:{v}"),
        }
    }
}

impl Attribute for ConstantValue {}
